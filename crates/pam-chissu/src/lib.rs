use std::ffi::{c_void, CStr, CString};
use std::fs;
use std::io;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr;
use std::slice;
use std::thread::sleep;
use std::time::{Duration, Instant};

use chissu_face_core::capture::{capture_frame_in_memory, CaptureConfig, DeviceLocator};
use chissu_face_core::errors::AppError;
use chissu_face_core::faces::{
    cosine_similarity, load_enrolled_descriptors, validate_user_name, DlibBackend,
    EnrolledDescriptor, FaceEmbeddingBackend, FaceExtractionConfig,
};
use image::{Rgb, RgbImage};
use libc::{c_int, free};
use pam_sys::{
    get_item, get_user, ConvClosure, PamConversation, PamHandle, PamItemType, PamMessage,
    PamMessageStyle, PamResponse, PamReturnCode,
};
use serde::Deserialize;
use syslog::{Facility, Formatter3164, Logger, LoggerBackend};
use thiserror::Error;

type PamResult<T> = Result<T, AuthError>;

const PRIMARY_CONFIG_PATH: &str = "/etc/chissu-pam/config.toml";
const SECONDARY_CONFIG_PATH: &str = "/usr/local/etc/chissu-pam/config.toml";
const DEFAULT_THRESHOLD: f64 = 0.7;
const DEFAULT_TIMEOUT_SECS: u64 = 5;
const DEFAULT_INTERVAL_MILLIS: u64 = 500;
const DEFAULT_VIDEO_DEVICE: &str = "/dev/video0";
const DEFAULT_STORE_DIR: &str = "/var/lib/chissu-pam/models";
const DEFAULT_PIXEL_FORMAT: &str = "Y16";
const DEFAULT_WARMUP_FRAMES: u32 = 0;
const DEFAULT_JITTERS: u32 = 1;

#[derive(Debug, Error)]
enum AuthError {
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Pam(String),
    #[error(transparent)]
    Core(#[from] AppError),
}

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    similarity_threshold: Option<f64>,
    capture_timeout_secs: Option<u64>,
    frame_interval_millis: Option<u64>,
    descriptor_store_dir: Option<PathBuf>,
    video_device: Option<String>,
    pixel_format: Option<String>,
    warmup_frames: Option<u32>,
    jitters: Option<u32>,
    landmark_model: Option<PathBuf>,
    encoder_model: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct ResolvedConfig {
    similarity_threshold: f64,
    capture_timeout: Duration,
    frame_interval: Duration,
    descriptor_store_dir: PathBuf,
    video_device: String,
    pixel_format: String,
    warmup_frames: u32,
    jitters: u32,
    landmark_model: Option<PathBuf>,
    encoder_model: Option<PathBuf>,
}

impl ResolvedConfig {
    fn from_raw(raw: ConfigFile) -> Self {
        Self {
            similarity_threshold: raw.similarity_threshold.unwrap_or(DEFAULT_THRESHOLD),
            capture_timeout: Duration::from_secs(
                raw.capture_timeout_secs
                    .unwrap_or(DEFAULT_TIMEOUT_SECS)
                    .max(1),
            ),
            frame_interval: Duration::from_millis(
                raw.frame_interval_millis.unwrap_or(DEFAULT_INTERVAL_MILLIS),
            ),
            descriptor_store_dir: raw
                .descriptor_store_dir
                .unwrap_or_else(|| PathBuf::from(DEFAULT_STORE_DIR)),
            video_device: raw
                .video_device
                .unwrap_or_else(|| DEFAULT_VIDEO_DEVICE.to_string()),
            pixel_format: raw
                .pixel_format
                .unwrap_or_else(|| DEFAULT_PIXEL_FORMAT.to_string()),
            warmup_frames: raw.warmup_frames.unwrap_or(DEFAULT_WARMUP_FRAMES),
            jitters: raw.jitters.unwrap_or(DEFAULT_JITTERS),
            landmark_model: raw.landmark_model,
            encoder_model: raw.encoder_model,
        }
    }
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        Self::from_raw(ConfigFile::default())
    }
}

struct LoadedConfig {
    resolved: ResolvedConfig,
    source: Option<PathBuf>,
}

#[derive(Debug)]
struct PamRequest {
    user: String,
}

struct PamConversationMessenger {
    conv: Option<ConvClosure>,
    data_ptr: *mut c_void,
}

impl PamConversationMessenger {
    unsafe fn new(pamh: *mut PamHandle, logger: &mut PamLogger) -> Self {
        if pamh.is_null() {
            logger.warn("PAM handle was null; conversation messages disabled");
            return Self::without_callback();
        }

        let handle = &*pamh;
        let mut ptr: *const c_void = ptr::null();
        let rc = get_item(handle, PamItemType::CONV, &mut ptr);
        if rc != PamReturnCode::SUCCESS {
            logger.warn(&format!("pam_get_item(PAM_CONV) failed: {rc}"));
            return Self::without_callback();
        }
        if ptr.is_null() {
            logger.warn("PAM provided no conversation struct; interactive hints disabled");
            return Self::without_callback();
        }

        let conv_struct = &*(ptr as *const PamConversation);
        match conv_struct.conv {
            Some(callback) => Self {
                conv: Some(callback),
                data_ptr: conv_struct.data_ptr,
            },
            None => {
                logger
                    .warn("PAM conversation struct lacked a callback; interactive hints disabled");
                Self::without_callback()
            }
        }
    }

    fn without_callback() -> Self {
        Self {
            conv: None,
            data_ptr: ptr::null_mut(),
        }
    }

    fn send_text_info(&mut self, logger: &mut PamLogger, message: &str) {
        self.send(logger, PamMessageStyle::TEXT_INFO, message);
    }

    fn send_error_msg(&mut self, logger: &mut PamLogger, message: &str) {
        self.send(logger, PamMessageStyle::ERROR_MSG, message);
    }

    fn send(&mut self, logger: &mut PamLogger, style: PamMessageStyle, message: &str) {
        let callback = match self.conv {
            Some(conv) => conv,
            None => return,
        };

        let Ok(c_message) = CString::new(message) else {
            logger.warn("PAM conversation message contained an interior null byte; skipped");
            return;
        };

        let mut pam_message = PamMessage {
            msg_style: style as c_int,
            msg: c_message.as_ptr(),
        };
        let mut pam_message_ptr: *mut PamMessage = &mut pam_message;
        let mut response_ptr: *mut PamResponse = ptr::null_mut();
        let status = callback(1, &mut pam_message_ptr, &mut response_ptr, self.data_ptr);
        unsafe {
            if !response_ptr.is_null() {
                let responses = slice::from_raw_parts_mut(response_ptr, 1);
                for response in responses {
                    if !response.resp.is_null() {
                        free(response.resp as *mut c_void);
                    }
                }
                free(response_ptr as *mut c_void);
            }
        }

        if status != PamReturnCode::SUCCESS as c_int {
            let code = PamReturnCode::from(status);
            logger.warn(&format!(
                "PAM conversation callback returned {code:?} while sending {style:?}"
            ));
        }
    }

    #[cfg(test)]
    fn from_callback(callback: ConvClosure) -> Self {
        Self {
            conv: Some(callback),
            data_ptr: ptr::null_mut(),
        }
    }
}

struct PamLogger {
    service: String,
    logger: Option<Logger<LoggerBackend, Formatter3164>>,
}

impl PamLogger {
    fn new(service: &str) -> Self {
        let formatter = Formatter3164 {
            facility: Facility::LOG_AUTHPRIV,
            hostname: None,
            process: "pam_chissu".into(),
            pid: 0,
        };
        let logger = syslog::unix(formatter.clone()).ok();
        Self {
            service: service.to_string(),
            logger,
        }
    }

    fn info(&mut self, message: &str) {
        self.send(|logger, msg| logger.info(msg), "INFO", message);
    }

    fn warn(&mut self, message: &str) {
        self.send(|logger, msg| logger.warning(msg), "WARN", message);
    }

    fn error(&mut self, message: &str) {
        self.send(|logger, msg| logger.err(msg), "ERROR", message);
    }

    fn debug(&mut self, message: &str) {
        self.send(|logger, msg| logger.debug(msg), "DEBUG", message);
    }

    fn send<F>(&mut self, mut emit: F, level: &str, message: &str)
    where
        F: FnMut(&mut Logger<LoggerBackend, Formatter3164>, &str) -> syslog::Result<()>,
    {
        let formatted = format!("[service={}] {}", self.service, message);
        if let Some(logger) = self.logger.as_mut() {
            let _ = emit(logger, &formatted);
        } else {
            eprintln!("pam_chissu {level}: {formatted}");
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum FailureReason {
    DescriptorsMissing,
    NoFaceDetected,
    ThresholdNotReached,
}

#[derive(Debug)]
struct AuthResult {
    success: bool,
    frames_captured: usize,
    best_similarity: f64,
    failure_reason: Option<FailureReason>,
}

impl AuthResult {
    fn success(best_similarity: f64, frames: usize) -> Self {
        Self {
            success: true,
            frames_captured: frames,
            best_similarity,
            failure_reason: None,
        }
    }

    fn failure(reason: FailureReason, best_similarity: f64, frames: usize) -> Self {
        Self {
            success: false,
            frames_captured: frames,
            best_similarity,
            failure_reason: Some(reason),
        }
    }
}

/// # Safety
/// The PAM stack guarantees `pamh` is a valid pointer for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn pam_sm_authenticate(
    pamh: *mut PamHandle,
    _flags: c_int,
    _argc: c_int,
    _argv: *const *const c_char,
) -> c_int {
    let (service, service_err) = match unsafe { get_service_name(pamh) } {
        Ok(name) => (name, None),
        Err(err) => ("unknown".to_string(), Some(err)),
    };
    let mut logger = PamLogger::new(&service);
    if let Some(err) = service_err {
        logger.warn(&format!("Failed to read PAM service name: {err}"));
    }

    let user = match unsafe { get_user_name(pamh) } {
        Ok(user) => user,
        Err(err) => {
            logger.error(&format!("Failed to read PAM user: {err}"));
            return PamReturnCode::SYSTEM_ERR as c_int;
        }
    };

    let mut messenger = PamConversationMessenger::new(pamh, &mut logger);

    let request = PamRequest { user };
    logger.info(&format!(
        "Starting face authentication for user '{}'.",
        request.user
    ));

    let outcome = match authenticate_user(&request, &mut logger, &mut messenger) {
        Ok(result) => result,
        Err(err) => {
            logger.error(&format!("Authentication aborted: {err}"));
            return PamReturnCode::SYSTEM_ERR as c_int;
        }
    };

    if outcome.success {
        logger.info(&format!(
            "Authentication success (frames={}, best_similarity={:.4}).",
            outcome.frames_captured, outcome.best_similarity
        ));
        messenger.send_text_info(
            &mut logger,
            &format!(
                "Face authentication succeeded for user '{}' via service '{}'.",
                request.user, service
            ),
        );
        PamReturnCode::SUCCESS as c_int
    } else {
        let reason = match outcome
            .failure_reason
            .unwrap_or(FailureReason::ThresholdNotReached)
        {
            FailureReason::DescriptorsMissing => "no enrolled descriptors",
            FailureReason::NoFaceDetected => "no face detected in captured frames",
            FailureReason::ThresholdNotReached => "no descriptor met similarity threshold",
        };
        logger.warn(&format!(
            "Authentication failed: {} (frames={}, best_similarity={:.4}).",
            reason, outcome.frames_captured, outcome.best_similarity
        ));
        let prompt = match outcome
            .failure_reason
            .unwrap_or(FailureReason::ThresholdNotReached)
        {
            FailureReason::DescriptorsMissing => format!(
                "Face authentication unavailable: no enrolled descriptors for '{}'.",
                request.user
            ),
            FailureReason::NoFaceDetected => {
                "No face detected before timeout; stay in frame and retry.".to_string()
            }
            FailureReason::ThresholdNotReached => {
                "Face detected but similarity below threshold; please retry.".to_string()
            }
        };
        messenger.send_error_msg(&mut logger, &prompt);
        PamReturnCode::AUTH_ERR as c_int
    }
}

/// # Safety
/// The PAM stack guarantees `pamh` (even if unused) remains a valid pointer for the call duration.
#[no_mangle]
pub unsafe extern "C" fn pam_sm_setcred(
    _pamh: *mut PamHandle,
    _flags: c_int,
    _argc: c_int,
    _argv: *const *const c_char,
) -> c_int {
    PamReturnCode::SUCCESS as c_int
}

fn authenticate_user(
    request: &PamRequest,
    logger: &mut PamLogger,
    messenger: &mut PamConversationMessenger,
) -> PamResult<AuthResult> {
    validate_user_name(&request.user)?;

    let LoadedConfig {
        resolved: config,
        source,
    } = load_config()?;
    if let Some(path) = source {
        logger.info(&format!("Loaded configuration from {}", path.display()));
    } else {
        logger.info("No configuration file found; using built-in defaults");
    }

    let descriptors =
        load_enrolled_descriptors(Some(config.descriptor_store_dir.as_path()), &request.user)?;
    if descriptors.is_empty() {
        return Ok(AuthResult::failure(
            FailureReason::DescriptorsMissing,
            f64::NEG_INFINITY,
            0,
        ));
    }

    let descriptor_len = verify_enrolled_descriptors(&descriptors)?;

    let capture_config = build_capture_config(&config);
    let embedder = build_embedder(&config)?;

    let deadline = Instant::now() + config.capture_timeout;
    let mut frames_captured = 0usize;
    let mut best_similarity = f64::NEG_INFINITY;
    let mut detected_any_face = false;
    let mut retry_hint_sent = false;

    while Instant::now() < deadline {
        frames_captured += 1;
        match capture_frame_in_memory(&capture_config) {
            Ok(frame) => {
                logger.debug(&format!(
                    "Captured frame {} from {} ({}x{})",
                    frames_captured, frame.device.path, frame.format.width, frame.format.height
                ));
                for entry in &frame.logs {
                    logger.debug(entry);
                }

                let rgb = gray_to_rgb(&frame.image);
                let faces = embedder.extract(&rgb, config.jitters)?;
                if faces.is_empty() {
                    logger.debug("No faces detected in frame");
                    if !retry_hint_sent {
                        messenger.send_error_msg(
                            logger,
                            "No face detected yet; align with the camera while we retry...",
                        );
                        retry_hint_sent = true;
                    }
                } else {
                    detected_any_face = true;
                    for rec in faces {
                        if rec.descriptor.len() != descriptor_len {
                            return Err(AuthError::Config(format!(
                                "Descriptor length mismatch: enrolled {} vs captured {}",
                                descriptor_len,
                                rec.descriptor.len()
                            )));
                        }
                        let similarity =
                            best_similarity_against_store(&rec.descriptor, &descriptors);
                        if similarity > best_similarity {
                            best_similarity = similarity;
                        }
                        if similarity >= config.similarity_threshold {
                            logger.info(&format!(
                        "Detected matching descriptor (similarity={similarity:.4}) after {frames_captured} frame(s)"
                    ));
                            return Ok(AuthResult::success(similarity, frames_captured));
                        }
                    }
                }
            }
            Err(err) => {
                logger.error(&format!("Failed to capture frame: {err}"));
                return Err(AuthError::Core(err));
            }
        }

        let now = Instant::now();
        if now >= deadline {
            break;
        }
        if config.frame_interval > Duration::ZERO {
            let remaining = deadline.saturating_duration_since(now);
            let sleep_for = if config.frame_interval < remaining {
                config.frame_interval
            } else {
                remaining
            };
            if sleep_for > Duration::ZERO {
                sleep(sleep_for);
            }
        }
    }

    let reason = if !detected_any_face {
        FailureReason::NoFaceDetected
    } else {
        FailureReason::ThresholdNotReached
    };
    Ok(AuthResult::failure(
        reason,
        best_similarity,
        frames_captured,
    ))
}

fn verify_enrolled_descriptors(descriptors: &[EnrolledDescriptor]) -> PamResult<usize> {
    let expected = descriptors
        .first()
        .ok_or_else(|| AuthError::Config("descriptor store unexpectedly empty".into()))?
        .descriptor
        .len();
    if expected == 0 {
        return Err(AuthError::Config(
            "stored descriptors have zero length".into(),
        ));
    }
    for record in descriptors {
        if record.descriptor.len() != expected {
            return Err(AuthError::Config(format!(
                "descriptor length mismatch: expected {}, found {}",
                expected,
                record.descriptor.len()
            )));
        }
    }
    Ok(expected)
}

fn best_similarity_against_store(candidate: &[f64], store: &[EnrolledDescriptor]) -> f64 {
    let mut best = f64::NEG_INFINITY;
    for record in store {
        let similarity = cosine_similarity(candidate, &record.descriptor);
        if similarity > best {
            best = similarity;
        }
    }
    best
}

fn build_capture_config(config: &ResolvedConfig) -> CaptureConfig {
    CaptureConfig {
        device: DeviceLocator::from_option(Some(config.video_device.clone())),
        pixel_format: config.pixel_format.clone(),
        width: None,
        height: None,
        exposure: None,
        gain: None,
        auto_exposure: false,
        auto_gain: false,
        warmup_frames: config.warmup_frames,
        output: None,
    }
}

fn build_embedder(config: &ResolvedConfig) -> PamResult<DlibBackend> {
    let models = FaceExtractionConfig {
        image: PathBuf::new(),
        landmark_model: config.landmark_model.clone(),
        encoder_model: config.encoder_model.clone(),
        output: None,
        jitters: config.jitters,
    }
    .resolve_models()?;
    DlibBackend::new(&models).map_err(AuthError::from)
}

fn gray_to_rgb(image: &image::GrayImage) -> RgbImage {
    let mut rgb = RgbImage::new(image.width(), image.height());
    for (x, y, pixel) in rgb.enumerate_pixels_mut() {
        let v = image.get_pixel(x, y)[0];
        *pixel = Rgb([v, v, v]);
    }
    rgb
}

fn load_config() -> PamResult<LoadedConfig> {
    if let Some(raw) = try_read_config(PRIMARY_CONFIG_PATH)? {
        return Ok(LoadedConfig {
            resolved: ResolvedConfig::from_raw(raw),
            source: Some(PathBuf::from(PRIMARY_CONFIG_PATH)),
        });
    }

    if let Some(raw) = try_read_config(SECONDARY_CONFIG_PATH)? {
        return Ok(LoadedConfig {
            resolved: ResolvedConfig::from_raw(raw),
            source: Some(PathBuf::from(SECONDARY_CONFIG_PATH)),
        });
    }

    Ok(LoadedConfig {
        resolved: ResolvedConfig::default(),
        source: None,
    })
}

fn try_read_config(path: &str) -> PamResult<Option<ConfigFile>> {
    match fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents)
            .map(Some)
            .map_err(|err| AuthError::Config(format!("Failed to parse {path}: {err}"))),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(AuthError::Config(format!("Failed to read {path}: {err}"))),
    }
}

unsafe fn get_user_name(pamh: *mut PamHandle) -> PamResult<String> {
    if pamh.is_null() {
        return Err(AuthError::Pam("pam handle was null".into()));
    }
    let handle = &*pamh;
    let mut ptr: *const c_char = ptr::null();
    let rc = get_user(handle, &mut ptr, ptr::null());
    if rc != PamReturnCode::SUCCESS {
        return Err(AuthError::Pam(format!("pam_get_user failed: {rc}")));
    }
    if ptr.is_null() {
        return Err(AuthError::Pam("pam_get_user returned null".into()));
    }
    Ok(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

unsafe fn get_service_name(pamh: *mut PamHandle) -> PamResult<String> {
    if pamh.is_null() {
        return Err(AuthError::Pam("pam handle was null".into()));
    }
    let handle = &*pamh;
    let mut ptr: *const std::ffi::c_void = ptr::null();
    let rc = get_item(handle, PamItemType::SERVICE, &mut ptr);
    if rc != PamReturnCode::SUCCESS {
        return Err(AuthError::Pam(format!(
            "pam_get_item(PAM_SERVICE) failed: {rc}"
        )));
    }
    if ptr.is_null() {
        return Err(AuthError::Pam("PAM service item was null".into()));
    }
    let cstr = CStr::from_ptr(ptr as *const c_char);
    Ok(cstr.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chissu_face_core::faces::BoundingBox;
    use std::ffi::CStr;
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};
    use tempfile::NamedTempFile;

    static CONV_LOG: OnceLock<Mutex<Vec<(PamMessageStyle, String)>>> = OnceLock::new();

    fn conversation_log() -> &'static Mutex<Vec<(PamMessageStyle, String)>> {
        CONV_LOG.get_or_init(|| Mutex::new(Vec::new()))
    }

    extern "C" fn recording_conv(
        num_msg: c_int,
        msg: *mut *mut PamMessage,
        resp: *mut *mut PamResponse,
        _data: *mut c_void,
    ) -> c_int {
        assert_eq!(num_msg, 1);
        unsafe {
            let message_ptr = *msg;
            assert!(!message_ptr.is_null());
            let style = PamMessageStyle::from((*message_ptr).msg_style as i32);
            let text = CStr::from_ptr((*message_ptr).msg)
                .to_string_lossy()
                .into_owned();
            conversation_log().lock().unwrap().push((style, text));
            if !resp.is_null() {
                *resp = ptr::null_mut();
            }
        }
        PamReturnCode::SUCCESS as c_int
    }

    #[test]
    fn resolved_config_defaults() {
        let loaded = ResolvedConfig::default();
        assert_eq!(loaded.similarity_threshold, DEFAULT_THRESHOLD);
        assert_eq!(
            loaded.capture_timeout,
            Duration::from_secs(DEFAULT_TIMEOUT_SECS)
        );
        assert_eq!(
            loaded.frame_interval,
            Duration::from_millis(DEFAULT_INTERVAL_MILLIS)
        );
        assert_eq!(loaded.video_device, DEFAULT_VIDEO_DEVICE);
        assert_eq!(loaded.pixel_format, DEFAULT_PIXEL_FORMAT);
        assert_eq!(
            loaded.descriptor_store_dir,
            PathBuf::from(DEFAULT_STORE_DIR)
        );
    }

    #[test]
    fn verify_enrolled_descriptors_detects_mismatch() {
        let descriptors = vec![
            EnrolledDescriptor {
                id: "a".into(),
                descriptor: vec![0.1, 0.2, 0.3],
                bounding_box: BoundingBox {
                    left: 0,
                    top: 0,
                    right: 1,
                    bottom: 1,
                },
                source: "input.json".into(),
                created_at: "2025-01-01T00:00:00Z".into(),
            },
            EnrolledDescriptor {
                id: "b".into(),
                descriptor: vec![0.1, 0.2],
                bounding_box: BoundingBox {
                    left: 0,
                    top: 0,
                    right: 1,
                    bottom: 1,
                },
                source: "input.json".into(),
                created_at: "2025-01-01T00:00:00Z".into(),
            },
        ];

        let err = verify_enrolled_descriptors(&descriptors).unwrap_err();
        assert!(
            matches!(err, AuthError::Config(msg) if msg.contains("descriptor length mismatch"))
        );
    }

    #[test]
    fn best_similarity_reports_peak_value() {
        let store = vec![
            EnrolledDescriptor {
                id: "a".into(),
                descriptor: vec![1.0, 0.0, 0.0],
                bounding_box: BoundingBox {
                    left: 0,
                    top: 0,
                    right: 1,
                    bottom: 1,
                },
                source: "a.json".into(),
                created_at: "2025-01-01T00:00:00Z".into(),
            },
            EnrolledDescriptor {
                id: "b".into(),
                descriptor: vec![0.0, 1.0, 0.0],
                bounding_box: BoundingBox {
                    left: 0,
                    top: 0,
                    right: 1,
                    bottom: 1,
                },
                source: "b.json".into(),
                created_at: "2025-01-01T00:00:00Z".into(),
            },
        ];

        let similarity = best_similarity_against_store(&[1.0, 0.0, 0.0], &store);
        assert!((similarity - 1.0).abs() < 1e-6);

        let similarity = best_similarity_against_store(&[0.0, 0.0, 1.0], &store);
        assert!(similarity.is_finite());
        assert!(similarity < 0.5);
    }

    #[test]
    fn try_read_config_parses_threshold() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "similarity_threshold = 0.8\nvideo_device = \"/dev/video5\""
        )
        .unwrap();

        let raw = try_read_config(file.path().to_str().unwrap())
            .unwrap()
            .unwrap();
        let resolved = ResolvedConfig::from_raw(raw);
        assert_eq!(resolved.similarity_threshold, 0.8);
        assert_eq!(resolved.video_device, "/dev/video5");
    }

    #[test]
    fn messenger_emits_text_and_error_messages() {
        conversation_log().lock().unwrap().clear();
        let mut messenger = PamConversationMessenger::from_callback(recording_conv);
        let mut logger = PamLogger::new("test-service");

        messenger.send_text_info(&mut logger, "Face authentication succeeded");
        messenger.send_error_msg(&mut logger, "Face authentication failed");

        let entries = conversation_log().lock().unwrap().clone();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, PamMessageStyle::TEXT_INFO);
        assert!(entries[0].1.contains("succeeded"));
        assert_eq!(entries[1].0, PamMessageStyle::ERROR_MSG);
        assert!(entries[1].1.contains("failed"));
    }
}
