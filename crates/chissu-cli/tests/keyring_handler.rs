use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use chissu_cli::cli::{KeyringCheckArgs, KeyringCommands, OutputMode};
use chissu_cli::commands::{CommandHandler, KeyringHandler};
use chissu_cli::errors::AppError;
use chissu_cli::keyring::KeyringCheckSummary;

fn sample_command() -> KeyringCommands {
    KeyringCommands::Check(KeyringCheckArgs {})
}

#[test]
fn keyring_handler_renders_summary() {
    let rendered = Arc::new(Mutex::new(Vec::new()));
    let handler = KeyringHandler::with_dependencies(
        sample_command(),
        |_cmd| {
            Ok(KeyringCheckSummary {
                user: "alice".into(),
                service: "service".into(),
            })
        },
        {
            let rendered = Arc::clone(&rendered);
            move |summary, mode| {
                rendered
                    .lock()
                    .unwrap()
                    .push((summary.user.clone(), matches!(mode, OutputMode::Json)));
                Ok(())
            }
        },
    );

    let code = handler.execute(OutputMode::Json, false).unwrap();
    assert_eq!(code, ExitCode::SUCCESS);
    let calls = rendered.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "alice");
    assert!(calls[0].1);
}

#[test]
fn keyring_handler_propagates_errors() {
    let handler = KeyringHandler::with_dependencies(
        sample_command(),
        |_cmd| Err(AppError::Capability("boom".into())),
        |_summary, _mode| Ok(()),
    );

    let err = handler.execute(OutputMode::Human, false).unwrap_err();
    match err {
        AppError::Capability(message) => assert_eq!(message, "boom"),
        other => panic!("unexpected error: {other}"),
    }
}
