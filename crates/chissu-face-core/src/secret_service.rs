use base64::{engine::general_purpose, Engine as _};
use keyring::{error::Error as KeyringError, Entry};
use rand::{rngs::OsRng, RngCore};
use thiserror::Error;

const DEFAULT_SERVICE_NAME: &str = "chissu-pam";
pub const AES_GCM_KEY_BYTES: usize = 32;

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

#[derive(Debug, Clone)]
pub enum DescriptorKeyStatus {
    Present(DescriptorKey),
    Missing,
}

#[derive(Debug, Clone)]
pub struct DescriptorKey {
    bytes: Vec<u8>,
}

impl DescriptorKey {
    pub fn generate() -> Self {
        let mut bytes = vec![0u8; AES_GCM_KEY_BYTES];
        OsRng.fill_bytes(&mut bytes);
        Self { bytes }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, DescriptorKeyLookupError> {
        Self::from_user_bytes("unknown", bytes)
    }

    pub fn from_user_bytes(user: &str, bytes: Vec<u8>) -> Result<Self, DescriptorKeyLookupError> {
        if bytes.len() != AES_GCM_KEY_BYTES {
            return Err(DescriptorKeyLookupError::InvalidFormat {
                user: user.to_string(),
                reason: format!(
                    "expected {AES_GCM_KEY_BYTES} bytes but found {}",
                    bytes.len()
                ),
            });
        }
        Ok(Self { bytes })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Debug, Error, Clone)]
pub enum DescriptorKeyLookupError {
    #[error(transparent)]
    SecretService(#[from] SecretServiceError),
    #[error("Secret Service entry for user '{user}' stored invalid AES-GCM key: {reason}")]
    InvalidFormat { user: String, reason: String },
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

pub fn fetch_descriptor_key(user: &str) -> Result<DescriptorKeyStatus, DescriptorKeyLookupError> {
    let entry = Entry::new(DEFAULT_SERVICE_NAME, user).map_err(|err| {
        SecretServiceError::new(
            user,
            DEFAULT_SERVICE_NAME,
            format!("failed to create keyring entry: {err}"),
        )
    })?;

    match entry.get_password() {
        Ok(secret) => decode_descriptor_key(user, &secret).map(DescriptorKeyStatus::Present),
        Err(KeyringError::NoEntry) => Ok(DescriptorKeyStatus::Missing),
        Err(err) => Err(DescriptorKeyLookupError::SecretService(
            SecretServiceError::new(user, DEFAULT_SERVICE_NAME, describe_keyring_error(&err)),
        )),
    }
}

pub fn store_descriptor_key(user: &str, key: &[u8]) -> Result<(), SecretServiceError> {
    let entry = Entry::new(DEFAULT_SERVICE_NAME, user).map_err(|err| {
        SecretServiceError::new(
            user,
            DEFAULT_SERVICE_NAME,
            format!("failed to create keyring entry: {err}"),
        )
    })?;

    entry
        .set_password(&general_purpose::STANDARD.encode(key))
        .map_err(|err| {
            SecretServiceError::new(user, DEFAULT_SERVICE_NAME, describe_keyring_error(&err))
        })
}

pub fn generate_descriptor_key() -> DescriptorKey {
    DescriptorKey::generate()
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

fn decode_descriptor_key(
    user: &str,
    secret: &str,
) -> Result<DescriptorKey, DescriptorKeyLookupError> {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return Err(DescriptorKeyLookupError::InvalidFormat {
            user: user.to_string(),
            reason: "stored secret was empty".into(),
        });
    }

    let decoded = general_purpose::STANDARD
        .decode(trimmed)
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(trimmed))
        .map_err(|err| DescriptorKeyLookupError::InvalidFormat {
            user: user.to_string(),
            reason: format!("base64 decode failed: {err}"),
        })?;

    DescriptorKey::from_user_bytes(user, decoded)
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

    #[test]
    fn decode_descriptor_key_accepts_padded_base64() {
        let raw = [0xABu8; AES_GCM_KEY_BYTES];
        let encoded = general_purpose::STANDARD.encode(raw);
        let key = decode_descriptor_key("alice", &encoded).unwrap();
        assert_eq!(key.as_bytes(), &raw);
    }

    #[test]
    fn decode_descriptor_key_rejects_short_values() {
        let encoded = general_purpose::STANDARD.encode([0xCDu8; 4]);
        let err = decode_descriptor_key("bob", &encoded).unwrap_err();
        match err {
            DescriptorKeyLookupError::InvalidFormat { user, reason } => {
                assert_eq!(user, "bob");
                assert!(reason.contains("expected"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }
}
