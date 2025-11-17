use std::path::PathBuf;

use chissu_cli::capture::CaptureHandler;
use chissu_cli::cli::{
    CaptureArgs, Commands, EnrollArgs, FaceExtractArgs, FacesCommands, KeyringCheckArgs,
    KeyringCommands,
};
use chissu_cli::commands::{
    CommandHandler, DoctorHandler, EnrollHandler, FacesHandler, KeyringHandler,
};

fn sample_capture_args() -> CaptureArgs {
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

fn sample_enroll_args() -> EnrollArgs {
    EnrollArgs {
        user: Some("alice".into()),
        store_dir: None,
        device: None,
        landmark_model: None,
        encoder_model: None,
        jitters: 1,
    }
}

fn sample_face_extract_args() -> FaceExtractArgs {
    FaceExtractArgs {
        image: PathBuf::from("image.png"),
        landmark_model: None,
        encoder_model: None,
        output: None,
        jitters: 1,
    }
}

fn assert_dispatch<T: 'static>(command: Commands)
where
    T: CommandHandler,
{
    let handler: Box<dyn CommandHandler> = command.into();
    assert!(handler.as_any().is::<T>());
}

#[test]
fn capture_command_dispatches_capture_handler() {
    assert_dispatch::<CaptureHandler>(Commands::Capture(sample_capture_args()));
}

#[test]
fn enroll_command_dispatches_enroll_handler() {
    assert_dispatch::<EnrollHandler>(Commands::Enroll(sample_enroll_args()));
}

#[test]
fn faces_command_dispatches_faces_handler() {
    assert_dispatch::<FacesHandler>(Commands::Faces(FacesCommands::Extract(
        sample_face_extract_args(),
    )));
}

#[test]
fn keyring_command_dispatches_keyring_handler() {
    assert_dispatch::<KeyringHandler>(Commands::Keyring(KeyringCommands::Check(
        KeyringCheckArgs {},
    )));
}

#[test]
fn doctor_command_dispatches_doctor_handler() {
    assert_dispatch::<DoctorHandler>(Commands::Doctor);
}
