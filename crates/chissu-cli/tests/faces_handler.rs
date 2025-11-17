use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use chissu_cli::cli::{FaceEnrollArgs, FaceExtractArgs, FacesCommands, OutputMode};
use chissu_cli::commands::{CommandHandler, FacesHandler, FacesHandlerDeps};
use chissu_cli::errors::AppError;
use chissu_cli::faces::{FaceExtractionOutcome, FaceExtractionSummary};

#[test]
fn faces_handler_extract_renders_success() {
    let render_calls = Arc::new(Mutex::new(0));
    let deps = FacesHandlerDeps::new(
        |_dir| Ok(None),
        |_config| {
            Ok(FaceExtractionOutcome {
                summary: FaceExtractionSummary {
                    success: true,
                    image_path: "img".into(),
                    output_path: "out".into(),
                    num_faces: 1,
                    faces: vec![],
                    landmark_model: "landmark".into(),
                    encoder_model: "encoder".into(),
                    num_jitters: 1,
                },
                logs: vec![],
            })
        },
        |_config| panic!("compare should not run"),
        |_config| panic!("enroll should not run"),
        |_config| panic!("remove should not run"),
        {
            let render_calls = Arc::clone(&render_calls);
            move |_outcome, _mode| {
                *render_calls.lock().unwrap() += 1;
                Ok(())
            }
        },
        |_outcome, _mode| panic!("compare render should not run"),
        |_outcome, _mode| panic!("enroll render should not run"),
        |_outcome, _mode| panic!("remove render should not run"),
    );

    let handler = FacesHandler::with_dependencies(
        FacesCommands::Extract(FaceExtractArgs {
            image: PathBuf::from("img.png"),
            landmark_model: None,
            encoder_model: None,
            output: None,
            jitters: 1,
        }),
        deps,
    );

    let code = handler.execute(OutputMode::Human, false).unwrap();
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(*render_calls.lock().unwrap(), 1);
}

#[test]
fn faces_handler_enroll_surfaces_errors() {
    let deps = FacesHandlerDeps::new(
        |dir| Ok(dir),
        |_config| panic!("extract should not run"),
        |_config| panic!("compare should not run"),
        |_config| Err(AppError::Capability("boom".into())),
        |_config| panic!("remove should not run"),
        |_outcome, _mode| Ok(()),
        |_outcome, _mode| Ok(()),
        |_outcome, _mode| Ok(()),
        |_outcome, _mode| Ok(()),
    );

    let handler = FacesHandler::with_dependencies(
        FacesCommands::Enroll(FaceEnrollArgs {
            user: "alice".into(),
            embedding: PathBuf::from("embedding.json"),
            store_dir: Some(PathBuf::from("/tmp")),
        }),
        deps,
    );

    let err = handler.execute(OutputMode::Json, false).unwrap_err();
    match err {
        AppError::Capability(message) => assert_eq!(message, "boom"),
        other => panic!("unexpected error: {other}"),
    }
}
