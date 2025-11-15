use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use image::ImageError;
use thiserror::Error;

use crate::secret_service::SecretServiceError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to open video device {device}: {source}")]
    DeviceOpen {
        device: String,
        #[source]
        source: io::Error,
    },

    #[error("input file not found or unreadable: {path}")]
    MissingInput { path: PathBuf },

    #[error("failed to decode image {path}: {source}")]
    ImageDecode {
        path: PathBuf,
        #[source]
        source: ImageError,
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

    #[error("missing {kind} model; provide {flag} or set ${env}")]
    MissingModel {
        kind: &'static str,
        flag: &'static str,
        env: &'static str,
    },

    #[error("failed to load model {path}: {message}")]
    ModelLoad { path: PathBuf, message: String },

    #[error("failed to write feature output {path}: {source}")]
    FeatureWrite {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to read feature file {path}: {source}")]
    FeatureRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("embedding file {path} is invalid: {message}")]
    InvalidFeatureFile { path: PathBuf, message: String },

    #[error("embedding payload in {path} failed validation: {message}")]
    EmbeddingValidation { path: PathBuf, message: String },

    #[error("invalid user name '{user}': {message}")]
    InvalidUser { user: String, message: String },

    #[error("no embedding with id {embedding_id} found for user {user}")]
    EmbeddingNotFound { user: String, embedding_id: String },

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("failed to read configuration file {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("invalid configuration file {path}: {message}")]
    ConfigParse { path: PathBuf, message: String },

    #[error("Secret Service unavailable for user {user} (service {service}): {message}")]
    SecretServiceUnavailable {
        user: String,
        service: String,
        message: String,
    },

    #[error("embedding store {path} is encrypted and requires a Secret Service key")]
    EncryptedStoreRequiresKey { path: PathBuf },

    #[error("embedding encryption error: {0}")]
    Encryption(String),

    #[error("Secret Service embedding key for user {user} invalid: {message}")]
    SecretServiceKeyInvalid { user: String, message: String },
}

impl AppError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            AppError::MissingInput { .. } => ExitCode::from(2),
            AppError::ImageDecode { .. } => ExitCode::from(2),
            AppError::UnsupportedFormat(_) => ExitCode::from(2),
            AppError::UnsupportedFrameSize { .. } => ExitCode::from(2),
            AppError::Capability(_) => ExitCode::from(3),
            AppError::DeviceOpen { .. } => ExitCode::from(4),
            AppError::MissingModel { .. } => ExitCode::from(2),
            AppError::ModelLoad { .. } => ExitCode::from(2),
            AppError::FeatureRead { .. } => ExitCode::from(2),
            AppError::InvalidFeatureFile { .. } => ExitCode::from(2),
            AppError::EmbeddingValidation { .. } => ExitCode::from(3),
            AppError::InvalidUser { .. } => ExitCode::from(2),
            AppError::EmbeddingNotFound { .. } => ExitCode::from(4),
            AppError::ConfigRead { .. } => ExitCode::from(2),
            AppError::ConfigParse { .. } => ExitCode::from(2),
            AppError::SecretServiceUnavailable { .. } => ExitCode::from(2),
            AppError::EncryptedStoreRequiresKey { .. } => ExitCode::from(2),
            AppError::Encryption(_) => ExitCode::from(2),
            AppError::SecretServiceKeyInvalid { .. } => ExitCode::from(2),
            _ => ExitCode::from(1),
        }
    }

    pub fn human_message(&self) -> String {
        self.to_string()
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl From<SecretServiceError> for AppError {
    fn from(err: SecretServiceError) -> Self {
        AppError::SecretServiceUnavailable {
            user: err.user().to_string(),
            service: err.service().to_string(),
            message: err.message().to_string(),
        }
    }
}

impl From<crate::secret_service::EmbeddingKeyLookupError> for AppError {
    fn from(err: crate::secret_service::EmbeddingKeyLookupError) -> Self {
        match err {
            crate::secret_service::EmbeddingKeyLookupError::SecretService(inner) => {
                AppError::from(inner)
            }
            crate::secret_service::EmbeddingKeyLookupError::InvalidFormat { user, reason } => {
                AppError::SecretServiceKeyInvalid {
                    user,
                    message: reason,
                }
            }
        }
    }
}
