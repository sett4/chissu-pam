use chissu_face_core::secret_service::{
    default_service_name, ensure_secret_service_available, KeyringSecretServiceProbe,
    SecretServiceProbe,
};

use crate::cli::OutputMode;
use crate::errors::AppResult;
use crate::output::render_keyring_check;

#[derive(Debug, Clone)]
pub struct KeyringCheckSummary {
    pub user: String,
    pub service: String,
}

pub fn run_keyring_check(mode: OutputMode) -> AppResult<()> {
    let user = whoami::username();
    let summary = check_with_probe(&KeyringSecretServiceProbe, user)?;
    render_keyring_check(&summary, mode)
}

pub fn check_with_probe<P: SecretServiceProbe>(
    probe: &P,
    user: String,
) -> AppResult<KeyringCheckSummary> {
    ensure_secret_service_available(probe, &user)?;
    Ok(KeyringCheckSummary {
        user,
        service: default_service_name().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::AppError;
    use chissu_face_core::secret_service::SecretServiceError;

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
    fn check_with_probe_returns_summary_on_success() {
        let probe = StubProbe { result: Ok(()) };
        let summary = check_with_probe(&probe, "alice".to_string()).unwrap();
        assert_eq!(summary.user, "alice");
        assert_eq!(summary.service, default_service_name());
    }

    #[test]
    fn check_with_probe_maps_error_to_app_error() {
        let probe = StubProbe {
            result: Err(SecretServiceError::new(
                "alice",
                default_service_name(),
                "locked",
            )),
        };
        let err = check_with_probe(&probe, "alice".to_string()).unwrap_err();
        match err {
            AppError::SecretServiceUnavailable {
                user,
                service,
                message,
            } => {
                assert_eq!(user, "alice");
                assert_eq!(service, default_service_name());
                assert!(message.contains("locked"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
