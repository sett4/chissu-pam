use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;

use crate::capture::{self, CaptureConfig, CaptureOutcome, DeviceLocator};
use crate::cli::{CaptureArgs, EnrollArgs, FaceEnrollArgs, FaceExtractArgs};
use crate::config::{self as config_loader, CaptureDefaults, FaceModelDefaults};
use crate::errors::{AppError, AppResult};
use crate::faces::{self, FaceEnrollmentConfig, FaceEnrollmentOutcome, FaceExtractionConfig};

#[derive(Debug)]
pub struct AutoEnrollOutcome {
    pub target_user: String,
    pub capture_path: PathBuf,
    pub descriptor_path: PathBuf,
    pub capture_deleted: bool,
    pub descriptor_deleted: bool,
    pub faces_detected: usize,
    pub enrollment: FaceEnrollmentOutcome,
    pub logs: Vec<String>,
}

#[derive(Clone, Debug)]
struct AutoEnrollContext {
    target_user: String,
    store_dir: Option<PathBuf>,
    capture_defaults: CaptureDefaults,
    device_override: Option<String>,
    landmark_model: Option<PathBuf>,
    encoder_model: Option<PathBuf>,
    jitters: u32,
    temp_base: PathBuf,
}

pub fn run_auto_enroll(args: &EnrollArgs) -> AppResult<AutoEnrollOutcome> {
    let target_user = resolve_target_user(args.user.as_deref())?;
    let store_dir = config_loader::resolve_store_dir(args.store_dir.clone())?;
    let capture_defaults = config_loader::load_capture_defaults()?;
    let model_defaults = config_loader::load_face_model_defaults()?;
    let (landmark_model, encoder_model) = resolve_model_paths(args, &model_defaults);
    let temp_base = std::env::temp_dir().join(format!("chissu-pam-{}", Uuid::new_v4()));

    let ctx = AutoEnrollContext {
        target_user,
        store_dir,
        capture_defaults,
        device_override: args.device.clone(),
        landmark_model,
        encoder_model,
        jitters: args.jitters,
        temp_base,
    };

    run_auto_enroll_with(
        ctx,
        capture::run_capture,
        faces::run_face_extraction,
        faces::run_face_enrollment,
    )
}

fn run_auto_enroll_with<Fc, Fe, Fm>(
    ctx: AutoEnrollContext,
    capture_runner: Fc,
    extractor: Fe,
    enroller: Fm,
) -> AppResult<AutoEnrollOutcome>
where
    Fc: Fn(&CaptureConfig) -> AppResult<CaptureOutcome>,
    Fe: Fn(&FaceExtractionConfig) -> AppResult<faces::FaceExtractionOutcome>,
    Fm: Fn(&FaceEnrollmentConfig) -> AppResult<FaceEnrollmentOutcome>,
{
    let (capture_path, descriptor_path) = build_temp_paths(&ctx.temp_base)?;

    let capture_args = CaptureArgs {
        device: ctx.device_override.clone(),
        pixel_format: None,
        width: None,
        height: None,
        exposure: None,
        gain: None,
        auto_exposure: false,
        auto_gain: false,
        warmup_frames: None,
        output: Some(capture_path.clone()),
    };
    let capture_config = capture::build_capture_config(&capture_args, &ctx.capture_defaults);

    let mut logs = Vec::new();
    logs.push(format!("Resolved target user: {}", ctx.target_user));
    logs.push(format!(
        "Resolved video device: {}",
        display_device(&capture_config.device)
    ));
    logs.push(format!(
        "Resolved pixel format: {}",
        capture_config.pixel_format
    ));
    logs.push(format!(
        "Resolved warm-up frames: {}",
        capture_config.warmup_frames
    ));

    let capture_outcome = capture_runner(&capture_config)?;
    logs.extend(capture_outcome.logs.clone());

    let extract_args = FaceExtractArgs {
        image: capture_path.clone(),
        landmark_model: ctx.landmark_model.clone(),
        encoder_model: ctx.encoder_model.clone(),
        output: Some(descriptor_path.clone()),
        jitters: ctx.jitters,
    };
    let extraction_config = FaceExtractionConfig::from(&extract_args);
    let extraction_outcome = extractor(&extraction_config)?;
    logs.extend(extraction_outcome.logs.clone());

    if extraction_outcome.summary.num_faces == 0 {
        return Err(AppError::DescriptorValidation {
            path: descriptor_path.clone(),
            message: "no faces detected in captured frame".into(),
        });
    }

    let enroll_args = FaceEnrollArgs {
        user: ctx.target_user.clone(),
        descriptor: descriptor_path.clone(),
        store_dir: ctx.store_dir.clone(),
    };
    let enrollment_config = FaceEnrollmentConfig::from(&enroll_args);
    let enrollment_outcome = enroller(&enrollment_config)?;
    logs.extend(enrollment_outcome.logs.clone());

    let capture_deleted = cleanup_file(&capture_path, &mut logs, "captured frame");
    let descriptor_deleted = cleanup_file(&descriptor_path, &mut logs, "descriptor payload");
    cleanup_dir(&ctx.temp_base, &mut logs);

    Ok(AutoEnrollOutcome {
        target_user: ctx.target_user,
        capture_path,
        descriptor_path,
        capture_deleted,
        descriptor_deleted,
        faces_detected: extraction_outcome.summary.num_faces,
        enrollment: enrollment_outcome,
        logs,
    })
}

fn build_temp_paths(base: &Path) -> AppResult<(PathBuf, PathBuf)> {
    fs::create_dir_all(base)?;
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let capture_path = base.join(format!("capture-{timestamp}.png"));
    let descriptor_path = base.join(format!("features-{timestamp}.json"));
    Ok((capture_path, descriptor_path))
}

fn cleanup_file(path: &Path, logs: &mut Vec<String>, label: &str) -> bool {
    match fs::remove_file(path) {
        Ok(_) => {
            logs.push(format!("Deleted temporary {label} {}", path.display()));
            true
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => false,
        Err(err) => {
            logs.push(format!(
                "Failed to delete temporary {label} {}: {err}",
                path.display()
            ));
            false
        }
    }
}

fn cleanup_dir(path: &Path, logs: &mut Vec<String>) {
    match fs::remove_dir(path) {
        Ok(_) => logs.push(format!("Removed temporary directory {}", path.display())),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) if err.kind() == io::ErrorKind::DirectoryNotEmpty => {
            logs.push(format!(
                "Temporary directory {} not empty; leaving in place",
                path.display()
            ));
        }
        Err(err) => logs.push(format!(
            "Failed to remove temporary directory {}: {err}",
            path.display()
        )),
    }
}

fn display_device(device: &DeviceLocator) -> String {
    match device {
        DeviceLocator::Index(idx) => format!("/dev/video{idx}"),
        DeviceLocator::Path(path) => path.display().to_string(),
    }
}

fn resolve_target_user(requested: Option<&str>) -> AppResult<String> {
    let current = whoami::username();
    let is_root = unsafe { libc::geteuid() == 0 };
    resolve_target_user_inner(requested, is_root, &current)
}

fn resolve_target_user_inner(
    requested: Option<&str>,
    is_root: bool,
    current: &str,
) -> AppResult<String> {
    match (requested, is_root) {
        (Some(user), false) => Err(AppError::InvalidUser {
            user: user.to_string(),
            message: "only root may override the enrollment user".into(),
        }),
        (Some(user), true) => Ok(user.to_string()),
        (None, _) => Ok(current.to_string()),
    }
}

fn resolve_model_paths(
    args: &EnrollArgs,
    defaults: &FaceModelDefaults,
) -> (Option<PathBuf>, Option<PathBuf>) {
    let landmark = args
        .landmark_model
        .clone()
        .or_else(|| defaults.landmark_model.clone());
    let encoder = args
        .encoder_model
        .clone()
        .or_else(|| defaults.encoder_model.clone());
    (landmark, encoder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{CaptureSummary, DeviceSummary, NegotiatedFormat};
    use crate::config::FaceModelDefaults;
    use crate::faces::{EnrollmentRecord, FaceExtractionOutcome, FaceExtractionSummary};
    use chrono::SecondsFormat;
    use std::cell::RefCell;
    use tempfile::tempdir;

    fn stub_capture_outcome(path: &Path) -> CaptureOutcome {
        CaptureOutcome {
            summary: CaptureSummary {
                success: true,
                output_path: path.display().to_string(),
                device: DeviceSummary {
                    driver: "v4l2-loopback".into(),
                    card: "stub".into(),
                    bus_info: "usb".into(),
                    path: path.display().to_string(),
                },
                format: NegotiatedFormat {
                    pixel_format: "Y16".into(),
                    width: 640,
                    height: 480,
                },
                exposure: None,
                gain: None,
                auto_exposure: Some("applied".into()),
                auto_gain: None,
            },
            logs: vec!["Stub capture".into()],
        }
    }

    fn stub_extraction_outcome(image: &Path, output: &Path, faces: usize) -> FaceExtractionOutcome {
        let mut face_records = Vec::new();
        for _ in 0..faces {
            face_records.push(crate::faces::FaceDescriptorRecord {
                bounding_box: crate::faces::BoundingBox {
                    left: 0,
                    top: 0,
                    right: 10,
                    bottom: 10,
                },
                descriptor: vec![0.0, 1.0],
            });
        }
        FaceExtractionOutcome {
            summary: FaceExtractionSummary {
                success: true,
                image_path: image.display().to_string(),
                output_path: output.display().to_string(),
                num_faces: faces,
                faces: face_records,
                landmark_model: "/models/landmark.dat".into(),
                encoder_model: "/models/encoder.dat".into(),
                num_jitters: 1,
            },
            logs: vec!["Stub extraction".into()],
        }
    }

    fn stub_enrollment_outcome(user: &str, store_path: &Path) -> FaceEnrollmentOutcome {
        FaceEnrollmentOutcome {
            user: user.to_string(),
            store_path: store_path.to_path_buf(),
            added: vec![EnrollmentRecord {
                id: "abc".into(),
                descriptor_len: 2,
                source: "stub".into(),
                created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            }],
            logs: vec!["Stub enrollment".into()],
        }
    }

    #[test]
    fn resolves_user_defaults_and_allows_root_override() {
        assert_eq!(
            resolve_target_user_inner(None, false, "alice").unwrap(),
            "alice"
        );
        assert!(resolve_target_user_inner(Some("bob"), false, "alice").is_err());
        assert_eq!(
            resolve_target_user_inner(Some("rooted"), true, "root").unwrap(),
            "rooted"
        );
    }

    #[test]
    fn uses_capture_defaults_when_overrides_missing() {
        let dir = tempdir().unwrap();
        let ctx = AutoEnrollContext {
            target_user: "alice".into(),
            store_dir: Some(dir.path().join("store")),
            capture_defaults: CaptureDefaults {
                device: Some("/dev/video5".into()),
                pixel_format: Some("GREY".into()),
                warmup_frames: Some(6),
            },
            device_override: None,
            landmark_model: None,
            encoder_model: None,
            jitters: 1,
            temp_base: dir.path().join("auto"),
        };

        let captured_path = RefCell::new(PathBuf::new());
        let capture_called = RefCell::new(false);

        let capture_runner = |config: &CaptureConfig| {
            *capture_called.borrow_mut() = true;
            if let DeviceLocator::Path(path) = &config.device {
                assert_eq!(path, &PathBuf::from("/dev/video5"));
            } else {
                panic!("expected path device");
            }
            assert_eq!(config.pixel_format, "GREY");
            assert_eq!(config.warmup_frames, 6);
            fs::write(config.output.as_ref().unwrap(), b"data").unwrap();
            *captured_path.borrow_mut() = config.output.as_ref().unwrap().clone();
            Ok(stub_capture_outcome(config.output.as_ref().unwrap()))
        };

        let extractor = |config: &FaceExtractionConfig| {
            fs::write(&config.image, b"image").unwrap();
            fs::write(config.output.as_ref().unwrap(), b"features").unwrap();
            Ok(stub_extraction_outcome(
                &config.image,
                config.output.as_ref().unwrap(),
                1,
            ))
        };

        let store_path = dir.path().join("store/alice.json");
        fs::create_dir_all(store_path.parent().unwrap()).unwrap();

        let enroller =
            |_config: &FaceEnrollmentConfig| Ok(stub_enrollment_outcome("alice", &store_path));

        let outcome = run_auto_enroll_with(ctx, capture_runner, extractor, enroller).unwrap();
        assert!(capture_called.into_inner());
        assert!(outcome.capture_deleted);
        assert!(outcome.descriptor_deleted);
        assert_eq!(outcome.faces_detected, 1);
        assert!(outcome
            .logs
            .iter()
            .any(|l| l.contains("Resolved video device")));
    }

    #[test]
    fn capture_errors_surface() {
        let dir = tempdir().unwrap();
        let ctx = AutoEnrollContext {
            target_user: "alice".into(),
            store_dir: None,
            capture_defaults: CaptureDefaults::default(),
            device_override: None,
            landmark_model: None,
            encoder_model: None,
            jitters: 1,
            temp_base: dir.path().join("auto"),
        };

        let err = run_auto_enroll_with(
            ctx,
            |_cfg| Err(AppError::Capability("failure".into())),
            |_cfg| unreachable!(),
            |_cfg| unreachable!(),
        );

        match err {
            Err(AppError::Capability(msg)) => assert_eq!(msg, "failure"),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn extractor_without_faces_aborts() {
        let dir = tempdir().unwrap();
        let ctx = AutoEnrollContext {
            target_user: "alice".into(),
            store_dir: None,
            capture_defaults: CaptureDefaults::default(),
            device_override: None,
            landmark_model: None,
            encoder_model: None,
            jitters: 1,
            temp_base: dir.path().join("auto"),
        };

        let capture_runner = |config: &CaptureConfig| {
            fs::write(config.output.as_ref().unwrap(), b"frame").unwrap();
            Ok(stub_capture_outcome(config.output.as_ref().unwrap()))
        };

        let extractor = |config: &FaceExtractionConfig| {
            fs::write(&config.image, b"image").unwrap();
            fs::write(config.output.as_ref().unwrap(), b"features").unwrap();
            Ok(stub_extraction_outcome(
                &config.image,
                config.output.as_ref().unwrap(),
                0,
            ))
        };

        let err = run_auto_enroll_with(ctx, capture_runner, extractor, |_cfg| unreachable!());
        assert!(matches!(err, Err(AppError::DescriptorValidation { .. })));
    }

    #[test]
    fn resolve_model_paths_prefers_cli_then_config() {
        let defaults = FaceModelDefaults {
            landmark_model: Some("/etc/landmark.dat".into()),
            encoder_model: Some("/etc/encoder.dat".into()),
        };
        let args = EnrollArgs {
            user: None,
            store_dir: None,
            device: None,
            landmark_model: Some("/tmp/landmark.dat".into()),
            encoder_model: None,
            jitters: 1,
        };

        let (landmark, encoder) = resolve_model_paths(&args, &defaults);
        assert_eq!(landmark.unwrap(), PathBuf::from("/tmp/landmark.dat"));
        assert_eq!(encoder.unwrap(), PathBuf::from("/etc/encoder.dat"));
    }
}
