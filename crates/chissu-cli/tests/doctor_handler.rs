use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use chissu_cli::cli::OutputMode;
use chissu_cli::commands::DoctorHandler;
use chissu_cli::doctor::{CheckStatus, DoctorCheck, DoctorOutcome};
use chissu_cli::errors::AppError;

fn sample_outcome(ok: bool) -> DoctorOutcome {
    DoctorOutcome {
        ok,
        checks: vec![DoctorCheck {
            name: "check".into(),
            status: if ok {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            },
            message: "msg".into(),
            path: None,
            device: None,
        }],
    }
}

#[test]
fn doctor_handler_uses_render_and_exit_code() {
    let renders = Arc::new(Mutex::new(0));
    let handler = DoctorHandler::with_dependencies(|| Ok(sample_outcome(false)), {
        let renders = Arc::clone(&renders);
        move |_outcome, _mode| {
            *renders.lock().unwrap() += 1;
            Ok(())
        }
    });

    let code = handler.execute(OutputMode::Human, false).unwrap();
    assert_eq!(code, ExitCode::from(1));
    assert_eq!(*renders.lock().unwrap(), 1);
}

#[test]
fn doctor_handler_propagates_errors() {
    let handler = DoctorHandler::with_dependencies(
        || Err(AppError::Capability("boom".into())),
        |_outcome, _mode| Ok(()),
    );

    let err = handler.execute(OutputMode::Json, false).unwrap_err();
    match err {
        AppError::Capability(message) => assert_eq!(message, "boom"),
        other => panic!("unexpected error: {other}"),
    }
}
