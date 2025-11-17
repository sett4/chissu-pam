use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use chissu_cli::capture::{
    CaptureHandler, CaptureOutcome, CaptureSummary, DeviceSummary, NegotiatedFormat,
};
use chissu_cli::cli::{CaptureArgs, OutputMode};
use chissu_cli::commands::CommandHandler;
use chissu_cli::config::CaptureDefaults;
use chissu_cli::errors::AppError;

fn sample_args() -> CaptureArgs {
    CaptureArgs {
        device: None,
        pixel_format: None,
        width: None,
        height: None,
        exposure: None,
        gain: None,
        auto_exposure: false,
        auto_gain: false,
        warmup_frames: None,
        output: None,
    }
}

fn sample_defaults() -> CaptureDefaults {
    CaptureDefaults {
        device: Some("/dev/video0".into()),
        pixel_format: Some("Y16".into()),
        warmup_frames: Some(4),
    }
}

fn sample_outcome() -> CaptureOutcome {
    CaptureOutcome {
        summary: CaptureSummary {
            success: true,
            output_path: "captures/test.png".into(),
            device: DeviceSummary {
                driver: "v4l".into(),
                card: "test".into(),
                bus_info: "usb".into(),
                path: "/dev/video0".into(),
            },
            format: NegotiatedFormat {
                pixel_format: "Y16".into(),
                width: 640,
                height: 480,
            },
            exposure: None,
            gain: None,
            auto_exposure: None,
            auto_gain: None,
        },
        logs: vec![],
    }
}

#[test]
fn capture_handler_renders_successful_capture() {
    let render_invocations = Arc::new(Mutex::new(Vec::new()));
    let handler = CaptureHandler::with_dependencies(
        sample_args(),
        || Ok(sample_defaults()),
        |_config| Ok(sample_outcome()),
        {
            let render_invocations = Arc::clone(&render_invocations);
            move |outcome, mode| {
                render_invocations.lock().unwrap().push((
                    outcome.summary.output_path.clone(),
                    matches!(mode, OutputMode::Json),
                ));
                Ok(())
            }
        },
    );

    let exit = handler.execute(OutputMode::Human, false).unwrap();
    assert_eq!(exit, ExitCode::SUCCESS);
    let calls = render_invocations.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "captures/test.png");
}

#[test]
fn capture_handler_propagates_runner_errors() {
    let handler = CaptureHandler::with_dependencies(
        sample_args(),
        || Ok(sample_defaults()),
        |_config| Err(AppError::Capability("boom".into())),
        |_outcome, _mode| Ok(()),
    );

    let err = handler.execute(OutputMode::Json, false).unwrap_err();
    match err {
        AppError::Capability(message) => assert_eq!(message, "boom"),
        other => panic!("unexpected error: {other}"),
    }
}
