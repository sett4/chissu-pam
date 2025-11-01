use std::io;
use std::process::ExitCode;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to open video device {device}: {source}")]
    DeviceOpen {
        device: String,
        #[source]
        source: io::Error,
    },

    #[error("device capability error: {0}")]
    Capability(String),

    #[error("unsupported pixel format '{0}' for selected device")]
    UnsupportedFormat(String),

    #[error("requested frame size {width}x{height} unsupported for pixel format {pixel_format}")]
    UnsupportedFrameSize {
        width: u32,
        height: u32,
        pixel_format: String,
    },

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("failed processing frame data: {0}")]
    FrameProcessing(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl AppError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            AppError::UnsupportedFormat(_) => ExitCode::from(2),
            AppError::UnsupportedFrameSize { .. } => ExitCode::from(2),
            AppError::Capability(_) => ExitCode::from(3),
            AppError::DeviceOpen { .. } => ExitCode::from(4),
            _ => ExitCode::from(1),
        }
    }

    pub fn human_message(&self) -> String {
        self.to_string()
    }
}

pub type AppResult<T> = Result<T, AppError>;
