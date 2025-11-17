use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use chissu_cli::auto_enroll::AutoEnrollOutcome;
use chissu_cli::cli::{EnrollArgs, OutputMode};
use chissu_cli::commands::{CommandHandler, EnrollHandler};
use chissu_cli::errors::AppError;
use chissu_cli::faces::{EnrollmentRecord, FaceEnrollmentOutcome};

fn sample_args() -> EnrollArgs {
    EnrollArgs {
        user: Some("alice".into()),
        store_dir: Some(PathBuf::from("/var/lib/chissu-pam")),
        device: None,
        landmark_model: None,
        encoder_model: None,
        jitters: 1,
    }
}

fn sample_outcome() -> AutoEnrollOutcome {
    AutoEnrollOutcome {
        target_user: "alice".into(),
        capture_path: PathBuf::from("captures/sample.png"),
        embedding_path: PathBuf::from("captures/features.json"),
        capture_deleted: true,
        embedding_deleted: true,
        faces_detected: 1,
        enrollment: FaceEnrollmentOutcome {
            user: "alice".into(),
            store_path: PathBuf::from("/var/lib/chissu-pam/alice.json"),
            added: vec![EnrollmentRecord {
                id: "abc".into(),
                embedding_len: 128,
                source: "captures/features.json".into(),
                created_at: "2024-01-01T00:00:00Z".into(),
            }],
            logs: vec!["enrolled".into()],
        },
        logs: vec!["ok".into()],
    }
}

#[test]
fn enroll_handler_passes_verbose_flag_to_renderer() {
    let render_calls = Arc::new(Mutex::new(Vec::new()));
    let handler = EnrollHandler::with_dependencies(sample_args(), |_args| Ok(sample_outcome()), {
        let render_calls = Arc::clone(&render_calls);
        move |outcome, mode, verbose| {
            render_calls
                .lock()
                .unwrap()
                .push((outcome.target_user.clone(), mode, verbose));
            Ok(())
        }
    });

    let exit = handler.execute(OutputMode::Json, true).unwrap();
    assert_eq!(exit, ExitCode::SUCCESS);
    let calls = render_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].2);
}

#[test]
fn enroll_handler_surfaces_run_errors() {
    let handler = EnrollHandler::with_dependencies(
        sample_args(),
        |_args| Err(AppError::Capability("boom".into())),
        |_outcome, _mode, _verbose| Ok(()),
    );

    let err = handler.execute(OutputMode::Human, false).unwrap_err();
    match err {
        AppError::Capability(message) => assert_eq!(message, "boom"),
        other => panic!("unexpected error: {other}"),
    }
}
