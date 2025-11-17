use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::{AppError, AppResult};
use crate::faces::extractor::{ensure_valid_faces, load_summary};
use crate::faces::store::{
    EnrolledEmbedding, EnvStoreDirResolver, FaceStore, FilesystemFaceStore, StoreDirResolver,
};
use crate::secret_service::{
    fetch_embedding_key, generate_embedding_key, store_embedding_key, EmbeddingKey,
    EmbeddingKeyStatus,
};

#[derive(Debug, Clone)]
pub struct FaceEnrollmentConfig {
    pub user: String,
    pub embedding: PathBuf,
    pub store_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct FaceRemovalConfig {
    pub user: String,
    pub embedding_ids: Vec<String>,
    pub remove_all: bool,
    pub store_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnrollmentRecord {
    pub id: String,
    #[serde(rename = "embedding_len")]
    pub embedding_len: usize,
    pub source: String,
    pub created_at: String,
}

#[derive(Debug)]
pub struct FaceEnrollmentOutcome {
    pub user: String,
    pub store_path: PathBuf,
    pub added: Vec<EnrollmentRecord>,
    pub logs: Vec<String>,
}

#[derive(Debug)]
pub struct FaceRemovalOutcome {
    pub user: String,
    pub store_path: PathBuf,
    pub removed_ids: Vec<String>,
    pub remaining: usize,
    pub cleared: bool,
    pub logs: Vec<String>,
}

pub trait KeyProvider {
    fn fetch(&self, user: &str) -> AppResult<EmbeddingKeyStatus>;
    fn store(&self, user: &str, key: &[u8]) -> AppResult<()>;
    fn generate(&self) -> EmbeddingKey;
}

#[derive(Clone, Copy, Default)]
pub struct SecretServiceKeyProvider;

impl KeyProvider for SecretServiceKeyProvider {
    fn fetch(&self, user: &str) -> AppResult<EmbeddingKeyStatus> {
        fetch_embedding_key(user).map_err(AppError::from)
    }

    fn store(&self, user: &str, key: &[u8]) -> AppResult<()> {
        store_embedding_key(user, key).map_err(AppError::from)
    }

    fn generate(&self) -> EmbeddingKey {
        generate_embedding_key()
    }
}

pub fn run_face_enrollment(config: &FaceEnrollmentConfig) -> AppResult<FaceEnrollmentOutcome> {
    let store = FilesystemFaceStore;
    let resolver = EnvStoreDirResolver;
    let keys = SecretServiceKeyProvider;
    run_face_enrollment_with(config, &store, &keys, &resolver)
}

pub fn run_face_enrollment_with<S, K, R>(
    config: &FaceEnrollmentConfig,
    store: &S,
    keys: &K,
    resolver: &R,
) -> AppResult<FaceEnrollmentOutcome>
where
    S: FaceStore,
    K: KeyProvider,
    R: StoreDirResolver,
{
    validate_user_name(&config.user)?;

    let mut logs = Vec::new();
    logs.push(format!(
        "Loading embedding payload from {}",
        config.embedding.display()
    ));

    let summary = load_summary(&config.embedding)
        .map_err(|err| map_to_embedding_validation(&config.embedding, err))?;
    let embedding_len = ensure_valid_faces(&summary.faces, &config.embedding)
        .map_err(|err| map_to_embedding_validation(&config.embedding, err))?;
    logs.push(format!(
        "Validated {} embedding(s) with length {}",
        summary.faces.len(),
        embedding_len
    ));

    let store_path = resolver
        .resolve(config.store_dir.as_deref())
        .join(format!("{}.json", config.user));
    let fetched_key = keys.fetch(&config.user)?;
    let current_key: Option<Vec<u8>> = match fetched_key {
        EmbeddingKeyStatus::Present(key) => Some(key.into_bytes()),
        EmbeddingKeyStatus::Missing => None,
    };

    let mut existing = store.load(&store_path, current_key.as_deref())?;

    if let Some(current_len) = existing.first().map(|entry| entry.embedding.len()) {
        if current_len != embedding_len {
            return Err(AppError::EmbeddingValidation {
                path: store_path.clone(),
                message: format!(
                    "embedding length mismatch with existing store (expected {current_len}, found {embedding_len})"
                ),
            });
        }
    }

    let mut added = Vec::with_capacity(summary.faces.len());
    for face in &summary.faces {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let record = EnrolledEmbedding {
            id: id.clone(),
            embedding: face.embedding.clone(),
            bounding_box: face.bounding_box.clone(),
            source: config.embedding.display().to_string(),
            created_at: created_at.clone(),
        };
        existing.push(record);
        added.push(EnrollmentRecord {
            id,
            embedding_len,
            source: config.embedding.display().to_string(),
            created_at,
        });
    }

    let new_key = keys.generate();
    store.save(&store_path, &existing, Some(new_key.as_bytes()))?;
    keys.store(&config.user, new_key.as_bytes())?;

    logs.push(format!(
        "Enrolled {} embedding(s) for user {}",
        added.len(),
        config.user
    ));
    logs.push(format!("Feature store: {}", store_path.display()));
    logs.push(format!(
        "Rotated Secret Service embedding key for user {}",
        config.user
    ));

    Ok(FaceEnrollmentOutcome {
        user: config.user.clone(),
        store_path,
        added,
        logs,
    })
}

pub fn run_face_removal(config: &FaceRemovalConfig) -> AppResult<FaceRemovalOutcome> {
    let store = FilesystemFaceStore;
    let resolver = EnvStoreDirResolver;
    let keys = SecretServiceKeyProvider;
    run_face_removal_with(config, &store, &keys, &resolver)
}

pub fn run_face_removal_with<S, K, R>(
    config: &FaceRemovalConfig,
    store: &S,
    keys: &K,
    resolver: &R,
) -> AppResult<FaceRemovalOutcome>
where
    S: FaceStore,
    K: KeyProvider,
    R: StoreDirResolver,
{
    validate_user_name(&config.user)?;

    let mut logs = Vec::new();
    let store_path = resolver
        .resolve(config.store_dir.as_deref())
        .join(format!("{}.json", config.user));
    let fetched_key = keys.fetch(&config.user)?;
    let key_bytes: Option<Vec<u8>> = match fetched_key {
        EmbeddingKeyStatus::Present(key) => Some(key.into_bytes()),
        EmbeddingKeyStatus::Missing => None,
    };

    let existing = store.load(&store_path, key_bytes.as_deref())?;
    logs.push(format!(
        "Loaded {} embedding(s) for user {}",
        existing.len(),
        config.user
    ));

    if config.remove_all {
        let removed_ids = existing
            .iter()
            .map(|entry| entry.id.clone())
            .collect::<Vec<_>>();
        store.delete(&store_path)?;
        logs.push(format!("Removed all embeddings for user {}", config.user));
        return Ok(FaceRemovalOutcome {
            user: config.user.clone(),
            store_path,
            removed_ids,
            remaining: 0,
            cleared: true,
            logs,
        });
    }

    if existing.is_empty() {
        if let Some(first) = config.embedding_ids.first() {
            return Err(AppError::EmbeddingNotFound {
                user: config.user.clone(),
                embedding_id: first.clone(),
            });
        }
    }

    let requested: HashSet<String> = config.embedding_ids.iter().cloned().collect();
    let mut retained = Vec::with_capacity(existing.len());
    let mut removed_ids = Vec::new();

    for entry in existing.into_iter() {
        if requested.contains(&entry.id) {
            removed_ids.push(entry.id.clone());
        } else {
            retained.push(entry);
        }
    }

    if removed_ids.len() != requested.len() {
        let removed_set: HashSet<&String> = removed_ids.iter().collect();
        if let Some(missing) = requested.iter().find(|id| !removed_set.contains(id)) {
            return Err(AppError::EmbeddingNotFound {
                user: config.user.clone(),
                embedding_id: missing.clone(),
            });
        }
    }

    store.save(&store_path, &retained, key_bytes.as_deref())?;
    logs.push(format!(
        "Removed {} embedding(s) for user {}",
        removed_ids.len(),
        config.user
    ));
    logs.push(format!(
        "Feature store now contains {} embedding(s)",
        retained.len()
    ));

    Ok(FaceRemovalOutcome {
        user: config.user.clone(),
        store_path,
        removed_ids,
        remaining: retained.len(),
        cleared: false,
        logs,
    })
}

pub fn validate_user_name(user: &str) -> AppResult<()> {
    if user.is_empty() {
        return Err(AppError::InvalidUser {
            user: user.to_string(),
            message: "user name cannot be empty".into(),
        });
    }

    if !user
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(AppError::InvalidUser {
            user: user.to_string(),
            message: "use ASCII letters, numbers, '-' or '_' only".into(),
        });
    }

    Ok(())
}

pub fn map_to_embedding_validation(path: &Path, err: AppError) -> AppError {
    match err {
        AppError::InvalidFeatureFile { message, .. } => AppError::EmbeddingValidation {
            path: path.to_path_buf(),
            message,
        },
        AppError::Serialization(source) => AppError::EmbeddingValidation {
            path: path.to_path_buf(),
            message: source.to_string(),
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use tempfile::TempDir;

    use crate::faces::extractor::{BoundingBox, FaceEmbeddingRecord, FaceExtractionSummary};
    use crate::faces::store::StoreDirResolver;

    #[test]
    fn enroll_creates_store_and_records_ids() {
        let tmp = TempDir::new().unwrap();
        let embedding_path = tmp.path().join("faces.json");
        let summary =
            summary_with_embeddings("input", vec![vec![1.0, 0.0, 0.5], vec![0.1, 0.2, 0.3]]);
        std::fs::write(
            &embedding_path,
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();

        let config = FaceEnrollmentConfig {
            user: "alice".into(),
            embedding: embedding_path.clone(),
            store_dir: Some(tmp.path().to_path_buf()),
        };
        let store = InMemoryStore::default();
        let keys = StubKeyProvider::new();
        let resolver = FixedStoreResolver(tmp.path().to_path_buf());
        let outcome = run_face_enrollment_with(&config, &store, &keys, &resolver).unwrap();
        assert_eq!(outcome.added.len(), 2);
        assert_eq!(store.saved.borrow().len(), 1);
        assert_eq!(keys.saved_keys.borrow().len(), 1);
    }

    #[test]
    fn removal_requires_existing_embedding_when_not_removing_all() {
        let tmp = TempDir::new().unwrap();
        let config = FaceRemovalConfig {
            user: "alice".into(),
            embedding_ids: vec!["missing".into()],
            remove_all: false,
            store_dir: Some(tmp.path().to_path_buf()),
        };
        let store = InMemoryStore::default();
        let keys = StubKeyProvider::new();
        let resolver = FixedStoreResolver(tmp.path().to_path_buf());
        let err = run_face_removal_with(&config, &store, &keys, &resolver).unwrap_err();
        assert!(matches!(err, AppError::EmbeddingNotFound { .. }));
    }

    #[derive(Default)]
    struct InMemoryStore {
        loaded: Vec<EnrolledEmbedding>,
        saved: RefCell<Vec<Vec<EnrolledEmbedding>>>,
    }

    impl FaceStore for InMemoryStore {
        fn load(&self, _path: &Path, _key: Option<&[u8]>) -> AppResult<Vec<EnrolledEmbedding>> {
            Ok(self.loaded.clone())
        }

        fn save(
            &self,
            _path: &Path,
            embeddings: &[EnrolledEmbedding],
            _key: Option<&[u8]>,
        ) -> AppResult<()> {
            self.saved.borrow_mut().push(embeddings.to_vec());
            Ok(())
        }

        fn delete(&self, _path: &Path) -> AppResult<()> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct FixedStoreResolver(PathBuf);

    impl StoreDirResolver for FixedStoreResolver {
        fn resolve(&self, _override_dir: Option<&Path>) -> PathBuf {
            self.0.clone()
        }
    }

    #[derive(Default)]
    struct StubKeyProvider {
        saved_keys: RefCell<Vec<Vec<u8>>>,
    }

    impl StubKeyProvider {
        fn new() -> Self {
            Self::default()
        }
    }

    impl KeyProvider for StubKeyProvider {
        fn fetch(&self, _user: &str) -> AppResult<EmbeddingKeyStatus> {
            Ok(EmbeddingKeyStatus::Missing)
        }

        fn store(&self, _user: &str, key: &[u8]) -> AppResult<()> {
            self.saved_keys.borrow_mut().push(key.to_vec());
            Ok(())
        }

        fn generate(&self) -> EmbeddingKey {
            EmbeddingKey::generate()
        }
    }

    fn summary_with_embeddings(source: &str, embeddings: Vec<Vec<f64>>) -> FaceExtractionSummary {
        FaceExtractionSummary {
            success: true,
            image_path: format!("{source}.png"),
            output_path: format!("{source}.json"),
            num_faces: embeddings.len(),
            faces: embeddings
                .into_iter()
                .map(|embedding| FaceEmbeddingRecord {
                    bounding_box: BoundingBox {
                        left: 0,
                        top: 0,
                        right: 1,
                        bottom: 1,
                    },
                    embedding,
                })
                .collect(),
            landmark_model: "landmark.dat".into(),
            encoder_model: "encoder.dat".into(),
            num_jitters: 1,
        }
    }
}
