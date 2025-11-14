mod secret_helper;

use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr;
use std::slice;
use std::thread::sleep;
use std::time::{Duration, Instant};

use chissu_config::{self, ConfigError, ResolvedConfig, ResolvedConfigWithSource};
use chissu_face_core::capture::{capture_frame_in_memory, CaptureConfig, DeviceLocator};
use chissu_face_core::errors::AppError;
use chissu_face_core::faces::{
    cosine_similarity, load_enrolled_descriptors, validate_user_name, DlibBackend,
    EnrolledDescriptor, FaceEmbeddingBackend, FaceExtractionConfig,
};
use chissu_face_core::secret_service::default_service_name;
use image::{Rgb, RgbImage};
use libc::{c_int, free};
use pam_sys::{
    get_item, get_user, ConvClosure, PamConversation, PamHandle, PamItemType, PamMessage,
    PamMessageStyle, PamResponse, PamReturnCode,
};
use secret_helper::{run_secret_service_helper, HelperError as SecretHelperError, HelperResponse};
use syslog::{Facility, Formatter3164, Logger, LoggerBackend};
use thiserror::Error;

type PamResult<T> = Result<T, AuthError>;

const SYSLOG_IDENTIFIER: &str = "pam_chissu";

#[derive(Debug, Error)]
enum AuthError {
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Pam(String),
    #[error(transparent)]
    Core(#[from] AppError),
    #[error("Secret Service unavailable: {0}")]
    SecretServiceUnavailable(String),
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
        let formatter = Self::formatter();
        let logger = syslog::unix(formatter.clone()).ok();
        Self {
            service: service.to_string(),
            logger,
        }
    }

    fn formatter() -> Formatter3164 {
        Formatter3164 {
            facility: Facility::LOG_AUTHPRIV,
            hostname: None,
            process: SYSLOG_IDENTIFIER.into(),
            pid: 0,
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
            eprintln!("{SYSLOG_IDENTIFIER} {level}: {formatted}");
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
        Err(AuthError::SecretServiceUnavailable(reason)) => {
            let message =
                format!("Secret Service unavailable; skipping face authentication: {reason}");
            logger.warn(&message);
            messenger.send_error_msg(
                &mut logger,
                &format!("Secret Service unavailable ({reason}); skipping face authentication.",),
            );
            return PamReturnCode::IGNORE as c_int;
        }
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

    let ResolvedConfigWithSource {
        resolved: config,
        source,
    } = load_config()?;
    if let Some(path) = source {
        logger.info(&format!("Loaded configuration from {}", path.display()));
    } else {
        logger.info("No configuration file found; using built-in defaults");
    }

    let mut descriptor_key: Option<Vec<u8>> = None;

    if config.require_secret_service {
        match run_secret_service_helper(&request.user, config.capture_timeout) {
            Ok(HelperResponse::Key(key_bytes)) => {
                logger.info(&format!(
                    "Secret Service helper returned descriptor key ({} bytes) for user '{}' via service '{}' — proceeding",
                    key_bytes.len(),
                    request.user,
                    default_service_name(),
                ));
                descriptor_key = Some(key_bytes);
            }
            Ok(HelperResponse::Missing { message }) => {
                logger.warn(&format!(
                    "Descriptor key missing for user '{}': {message}",
                    request.user
                ));
                return Ok(AuthResult::failure(
                    FailureReason::DescriptorsMissing,
                    f64::NEG_INFINITY,
                    0,
                ));
            }
            Err(SecretHelperError::SecretServiceUnavailable(message)) => {
                return Err(AuthError::SecretServiceUnavailable(message));
            }
            Err(SecretHelperError::IpcFailure(message)) => {
                return Err(AuthError::Pam(format!(
                    "Secret Service helper failed: {message}"
                )));
            }
        }
    } else {
        logger.info("Secret Service probe disabled via configuration; continuing without check");
    }

    let descriptors = load_descriptor_store(&config, request, logger, &mut descriptor_key)?;
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

fn load_descriptor_store(
    config: &ResolvedConfig,
    request: &PamRequest,
    logger: &mut PamLogger,
    descriptor_key: &mut Option<Vec<u8>>,
) -> PamResult<Vec<EnrolledDescriptor>> {
    loop {
        match load_enrolled_descriptors(
            Some(config.descriptor_store_dir.as_path()),
            &request.user,
            descriptor_key.as_deref(),
        ) {
            Ok(descriptors) => return Ok(descriptors),
            Err(AppError::EncryptedStoreRequiresKey { .. }) => {
                if descriptor_key.is_some() {
                    return Err(AuthError::SecretServiceUnavailable(
                        "Secret Service key failed to decrypt descriptor store".into(),
                    ));
                }
                match run_secret_service_helper(&request.user, config.capture_timeout) {
                    Ok(HelperResponse::Key(bytes)) => {
                        logger.info(&format!(
                            "Secret Service helper returned descriptor key ({} bytes) for user '{}' via service '{}' — retrying store load",
                            bytes.len(),
                            request.user,
                            default_service_name(),
                        ));
                        *descriptor_key = Some(bytes);
                    }
                    Ok(HelperResponse::Missing { message }) => {
                        logger.warn(&format!(
                            "Descriptor key missing for user '{}': {message}",
                            request.user
                        ));
                        return Err(AuthError::SecretServiceUnavailable(message));
                    }
                    Err(SecretHelperError::SecretServiceUnavailable(message)) => {
                        return Err(AuthError::SecretServiceUnavailable(message));
                    }
                    Err(SecretHelperError::IpcFailure(message)) => {
                        return Err(AuthError::Pam(format!(
                            "Secret Service helper failed: {message}"
                        )));
                    }
                }
            }
            Err(err) => return Err(AuthError::Core(err)),
        }
    }
}

fn load_config() -> PamResult<ResolvedConfigWithSource> {
    chissu_config::load_resolved_config().map_err(map_config_error)
}

fn map_config_error(err: ConfigError) -> AuthError {
    match err {
        ConfigError::Read { path, source } => {
            AuthError::Config(format!("Failed to read {}: {}", path.display(), source))
        }
        ConfigError::Parse { path, message } => {
            AuthError::Config(format!("Failed to parse {}: {}", path.display(), message))
        }
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
        assert_eq!(
            loaded.similarity_threshold,
            chissu_config::DEFAULT_SIMILARITY_THRESHOLD
        );
        assert_eq!(
            loaded.capture_timeout,
            Duration::from_secs(chissu_config::DEFAULT_TIMEOUT_SECS)
        );
        assert_eq!(
            loaded.frame_interval,
            Duration::from_millis(chissu_config::DEFAULT_INTERVAL_MILLIS)
        );
        assert_eq!(loaded.video_device, chissu_config::DEFAULT_VIDEO_DEVICE);
        assert_eq!(loaded.pixel_format, chissu_config::DEFAULT_PIXEL_FORMAT);
        assert_eq!(
            loaded.descriptor_store_dir,
            PathBuf::from(chissu_config::DEFAULT_STORE_DIR)
        );
        assert!(!loaded.require_secret_service);
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

        let loaded = chissu_config::load_from_paths(&[file.path().to_path_buf()])
            .unwrap()
            .unwrap();
        let raw = loaded.into_contents();
        let resolved = ResolvedConfig::from_raw(raw);
        assert_eq!(resolved.similarity_threshold, 0.8);
        assert_eq!(resolved.video_device, "/dev/video5");
    }

    #[test]
    fn try_read_config_disables_secret_service_when_requested() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "require_secret_service = false").unwrap();
        let loaded = chissu_config::load_from_paths(&[file.path().to_path_buf()])
            .unwrap()
            .unwrap();
        let raw = loaded.into_contents();
        let resolved = ResolvedConfig::from_raw(raw);
        assert!(!resolved.require_secret_service);
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

    #[test]
    fn pam_logger_formatter_uses_syslog_identifier() {
        let formatter = PamLogger::formatter();
        assert_eq!(formatter.process, SYSLOG_IDENTIFIER);
    }
}
