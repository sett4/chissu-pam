use std::any::Any;
use std::process::ExitCode;

use crate::capture::CaptureHandler;
use crate::cli::{Commands, OutputMode};
use crate::errors::AppResult;

pub trait CommandHandler: Send + Sync {
    fn execute(&self, mode: OutputMode, verbose: bool) -> AppResult<ExitCode>;
    fn as_any(&self) -> &dyn Any;
}

mod doctor;
mod enroll;
mod faces;
mod keyring;

pub use doctor::DoctorHandler;
pub use enroll::EnrollHandler;
pub use faces::{FacesHandler, FacesHandlerDeps};
pub use keyring::KeyringHandler;

impl From<Commands> for Box<dyn CommandHandler> {
    fn from(command: Commands) -> Self {
        match command {
            Commands::Capture(args) => Box::new(CaptureHandler::new(args)),
            Commands::Enroll(args) => Box::new(EnrollHandler::new(args)),
            Commands::Faces(cmd) => Box::new(FacesHandler::new(cmd)),
            Commands::Keyring(cmd) => Box::new(KeyringHandler::new(cmd)),
            Commands::Doctor => Box::new(DoctorHandler::new()),
        }
    }
}
