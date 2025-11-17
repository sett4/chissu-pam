use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose, Engine as _};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::errors::{AppError, AppResult};

const DEFAULT_STORE_DIR: &str = "/var/lib/chissu-pam/models";
const FEATURE_STORE_ENV: &str = "CHISSU_PAM_STORE_DIR";
const STORE_VERSION: u32 = 1;
const STORE_ALGORITHM: &str = "AES-256-GCM";
const STORE_NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnrolledEmbedding {
    pub id: String,
    #[serde(rename = "embedding")]
    pub embedding: Vec<f64>,
    pub bounding_box: crate::faces::extractor::BoundingBox,
    pub source: String,
    pub created_at: String,
}

pub trait FaceStore {
    fn load(&self, path: &Path, key: Option<&[u8]>) -> AppResult<Vec<EnrolledEmbedding>>;
    fn save(
        &self,
        path: &Path,
        embeddings: &[EnrolledEmbedding],
        key: Option<&[u8]>,
    ) -> AppResult<()>;
    fn delete(&self, path: &Path) -> AppResult<()>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FilesystemFaceStore;

impl FaceStore for FilesystemFaceStore {
    fn load(&self, path: &Path, key: Option<&[u8]>) -> AppResult<Vec<EnrolledEmbedding>> {
        read_enrolled_store(path, key)
    }

    fn save(
        &self,
        path: &Path,
        embeddings: &[EnrolledEmbedding],
        key: Option<&[u8]>,
    ) -> AppResult<()> {
        write_enrolled_store(path, embeddings, key)
    }

    fn delete(&self, path: &Path) -> AppResult<()> {
        if path.exists() {
            fs::remove_file(path).map_err(|source| AppError::FeatureWrite {
                path: path.to_path_buf(),
                source,
            })?;
        }
        Ok(())
    }
}

pub trait StoreDirResolver {
    fn resolve(&self, override_dir: Option<&Path>) -> PathBuf;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EnvStoreDirResolver;

impl StoreDirResolver for EnvStoreDirResolver {
    fn resolve(&self, override_dir: Option<&Path>) -> PathBuf {
        if let Some(dir) = override_dir {
            dir.to_path_buf()
        } else if let Ok(env_value) = env::var(FEATURE_STORE_ENV) {
            PathBuf::from(env_value)
        } else {
            PathBuf::from(DEFAULT_STORE_DIR)
        }
    }
}

pub fn user_store_path(store_dir: Option<&Path>, user: &str) -> PathBuf {
    let resolver = EnvStoreDirResolver;
    resolver.resolve(store_dir).join(format!("{user}.json"))
}

pub fn load_enrolled_embeddings(
    store_dir: Option<&Path>,
    user: &str,
    key: Option<&[u8]>,
) -> AppResult<Vec<EnrolledEmbedding>> {
    let path = user_store_path(store_dir, user);
    let store = FilesystemFaceStore;
    store.load(&path, key)
}

pub fn read_enrolled_store(path: &Path, key: Option<&[u8]>) -> AppResult<Vec<EnrolledEmbedding>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let data = fs::read(path).map_err(|source| AppError::FeatureRead {
        path: path.to_path_buf(),
        source,
    })?;

    if let Ok(wrapper) = serde_json::from_slice::<EncryptedEmbeddingStore>(&data) {
        return decrypt_encrypted_store(path, wrapper, key);
    }

    serde_json::from_slice(&data).map_err(|err| AppError::InvalidFeatureFile {
        path: path.to_path_buf(),
        message: format!("invalid feature store contents: {err}"),
    })
}

pub fn write_enrolled_store(
    path: &Path,
    embeddings: &[EnrolledEmbedding],
    key: Option<&[u8]>,
) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AppError::FeatureWrite {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = NamedTempFile::new_in(parent).map_err(|source| AppError::FeatureWrite {
        path: path.to_path_buf(),
        source,
    })?;

    {
        let file = tmp.as_file_mut();
        {
            let mut writer = BufWriter::new(&mut *file);
            let serialized = if let Some(key_bytes) = key {
                serialize_encrypted_store(embeddings, key_bytes)?
            } else {
                serde_json::to_vec_pretty(embeddings)?
            };
            writer
                .write_all(&serialized)
                .map_err(|source| AppError::FeatureWrite {
                    path: path.to_path_buf(),
                    source,
                })?;
            writer.write_all(b"\n").ok();
            writer.flush().map_err(|source| AppError::FeatureWrite {
                path: path.to_path_buf(),
                source,
            })?;
        }
        file.sync_all().map_err(|source| AppError::FeatureWrite {
            path: path.to_path_buf(),
            source,
        })?;
    }

    let file = tmp.persist(path).map_err(|err| AppError::FeatureWrite {
        path: path.to_path_buf(),
        source: err.error,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file
            .metadata()
            .map_err(|source| AppError::FeatureWrite {
                path: path.to_path_buf(),
                source,
            })?
            .permissions();
        perms.set_mode(0o600);
        file.set_permissions(perms)
            .map_err(|source| AppError::FeatureWrite {
                path: path.to_path_buf(),
                source,
            })?;
    }

    Ok(())
}

fn serialize_encrypted_store(embeddings: &[EnrolledEmbedding], key: &[u8]) -> AppResult<Vec<u8>> {
    let plaintext = serde_json::to_vec(embeddings)?;
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| AppError::Encryption("invalid AES-GCM key length".into()))?;
    let mut nonce = [0u8; STORE_NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_ref())
        .map_err(|err| AppError::Encryption(format!("failed to encrypt embedding store: {err}")))?;
    let wrapper = EncryptedEmbeddingStore {
        version: STORE_VERSION,
        algorithm: STORE_ALGORITHM.to_string(),
        nonce: general_purpose::STANDARD.encode(nonce),
        ciphertext: general_purpose::STANDARD.encode(ciphertext),
    };
    serde_json::to_vec_pretty(&wrapper).map_err(AppError::from)
}

fn decrypt_encrypted_store(
    path: &Path,
    wrapper: EncryptedEmbeddingStore,
    key: Option<&[u8]>,
) -> AppResult<Vec<EnrolledEmbedding>> {
    if wrapper.algorithm != STORE_ALGORITHM {
        return Err(AppError::Encryption(format!(
            "unsupported embedding store algorithm '{}'",
            wrapper.algorithm
        )));
    }
    if wrapper.version != STORE_VERSION {
        return Err(AppError::Encryption(format!(
            "unsupported embedding store version {}",
            wrapper.version
        )));
    }

    let key_bytes = key.ok_or_else(|| AppError::EncryptedStoreRequiresKey {
        path: path.to_path_buf(),
    })?;

    let nonce_bytes = general_purpose::STANDARD
        .decode(wrapper.nonce.trim())
        .map_err(|err| AppError::Encryption(format!("invalid nonce encoding: {err}")))?;
    if nonce_bytes.len() != STORE_NONCE_LEN {
        return Err(AppError::Encryption(format!(
            "expected nonce of {} bytes but found {}",
            STORE_NONCE_LEN,
            nonce_bytes.len()
        )));
    }

    let ciphertext = general_purpose::STANDARD
        .decode(wrapper.ciphertext.trim())
        .map_err(|err| AppError::Encryption(format!("invalid ciphertext encoding: {err}")))?;

    let cipher = Aes256Gcm::new_from_slice(key_bytes)
        .map_err(|_| AppError::Encryption("invalid AES-GCM key length".into()))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .map_err(|err| AppError::Encryption(format!("failed to decrypt embedding store: {err}")))?;

    serde_json::from_slice(&plaintext).map_err(|err| AppError::InvalidFeatureFile {
        path: path.to_path_buf(),
        message: format!("invalid decrypted feature store contents: {err}"),
    })
}

#[derive(Serialize, Deserialize)]
struct EncryptedEmbeddingStore {
    version: u32,
    algorithm: String,
    nonce: String,
    ciphertext: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{SecondsFormat, Utc};
    use tempfile::TempDir;

    #[test]
    fn filesystem_store_round_trip_without_encryption() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("faces.json");
        let store = FilesystemFaceStore;
        let embeddings = vec![dummy_embedding("source.json")];
        store.save(&path, &embeddings, None).unwrap();

        let loaded = store.load(&path, None).unwrap();
        assert_eq!(loaded, embeddings);
    }

    #[test]
    fn filesystem_store_round_trip_with_encryption() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("faces.json");
        let store = FilesystemFaceStore;
        let embeddings = vec![dummy_embedding("source.json")];
        let key = [0x22u8; 32];
        store.save(&path, &embeddings, Some(&key)).unwrap();

        let loaded = store.load(&path, Some(&key)).unwrap();
        assert_eq!(loaded, embeddings);
    }

    #[test]
    fn user_store_path_prefers_override_then_env() {
        let tmp = TempDir::new().unwrap();
        std::env::set_var(
            FEATURE_STORE_ENV,
            tmp.path().join("env").display().to_string(),
        );
        let override_dir = tmp.path().join("override");
        let path = user_store_path(Some(&override_dir), "alice");
        assert!(path.starts_with(&override_dir));
        std::env::remove_var(FEATURE_STORE_ENV);
    }

    fn dummy_embedding(source: &str) -> EnrolledEmbedding {
        EnrolledEmbedding {
            id: "id".into(),
            embedding: vec![0.1, 0.2, 0.3],
            bounding_box: crate::faces::extractor::BoundingBox {
                left: 0,
                top: 0,
                right: 1,
                bottom: 1,
            },
            source: source.into(),
            created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}
