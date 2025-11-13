use keyring::{error::Error as KeyringError, Entry};
use thiserror::Error;

const DEFAULT_SERVICE_NAME: &str = "chissu-pam";

#[derive(Debug, Error, Clone)]
#[error("Secret Service unavailable for user '{user}' (service '{service}'): {message}")]
pub struct SecretServiceError {
    user: String,
    service: String,
    message: String,
}

impl SecretServiceError {
    pub fn new(
        user: impl Into<String>,
        service: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            user: user.into(),
            service: service.into(),
            message: message.into(),
        }
    }

    pub fn user(&self) -> &str {
        &self.user
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

pub trait SecretServiceProbe {
    fn check(&self, user: &str) -> Result<(), SecretServiceError>;
}

#[derive(Debug, Clone, Copy)]
pub struct KeyringSecretServiceProbe;

impl SecretServiceProbe for KeyringSecretServiceProbe {
    fn check(&self, user: &str) -> Result<(), SecretServiceError> {
        let entry = Entry::new(DEFAULT_SERVICE_NAME, user).map_err(|err| {
            SecretServiceError::new(
                user,
                DEFAULT_SERVICE_NAME,
                format!("failed to create keyring entry: {err}"),
            )
        })?;
        match entry.get_password() {
            Ok(_) | Err(KeyringError::NoEntry) => Ok(()),
            Err(err) => Err(SecretServiceError::new(
                user,
                DEFAULT_SERVICE_NAME,
                describe_keyring_error(&err),
            )),
        }
    }
}

pub fn ensure_secret_service_available<P: SecretServiceProbe>(
    probe: &P,
    user: &str,
) -> Result<(), SecretServiceError> {
    probe.check(user)
}

pub fn default_service_name() -> &'static str {
    DEFAULT_SERVICE_NAME
}

fn describe_keyring_error(err: &KeyringError) -> String {
    match err {
        KeyringError::NoStorageAccess(inner) => {
            format!("Secret Service locked or unavailable: {inner}")
        }
        KeyringError::PlatformFailure(inner) => {
            format!("Secret Service platform failure: {inner}")
        }
        KeyringError::TooLong(attr, limit) => {
            format!("Secret Service attribute '{attr}' exceeded platform limit {limit}")
        }
        KeyringError::Invalid(attr, reason) => {
            format!("Secret Service attribute {attr} invalid: {reason}")
        }
        KeyringError::Ambiguous(items) => format!(
            "Secret Service returned {} matching credentials for probe entry",
            items.len()
        ),
        KeyringError::BadEncoding(_) => {
            "Secret Service returned a non-UTF8 secret for probe entry".into()
        }
        KeyringError::NoEntry => "Secret Service reported no entry".into(),
        _ => format!("Secret Service error: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct StubProbe {
        result: Result<(), SecretServiceError>,
    }

    impl SecretServiceProbe for StubProbe {
        fn check(&self, _user: &str) -> Result<(), SecretServiceError> {
            self.result.clone()
        }
    }

    #[test]
    fn ensure_secret_service_available_propagates_success() {
        let probe = StubProbe { result: Ok(()) };
        assert!(ensure_secret_service_available(&probe, "alice").is_ok());
    }

    #[test]
    fn ensure_secret_service_available_propagates_failure() {
        let probe = StubProbe {
            result: Err(SecretServiceError::new(
                "alice",
                DEFAULT_SERVICE_NAME,
                "locked",
            )),
        };
        let err = ensure_secret_service_available(&probe, "alice").unwrap_err();
        assert!(err.message().contains("locked"));
        assert_eq!(err.user(), "alice");
        assert_eq!(err.service(), DEFAULT_SERVICE_NAME);
    }
}
