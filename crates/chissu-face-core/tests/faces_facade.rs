use std::cell::RefCell;
use std::path::{Path, PathBuf};

use chissu_face_core::faces::enrollment::{
    run_face_enrollment_with, run_face_removal_with, FaceEnrollmentConfig, FaceRemovalConfig,
    KeyProvider,
};
use chissu_face_core::faces::extractor::{BoundingBox, FaceEmbeddingRecord, FaceExtractionSummary};
use chissu_face_core::faces::store::{EnrolledEmbedding, FaceStore, StoreDirResolver};
use chissu_face_core::errors::AppResult;
use chissu_face_core::secret_service::{EmbeddingKey, EmbeddingKeyStatus};
use tempfile::TempDir;

#[test]
fn integration_enroll_and_remove_with_stubs() {
    let tmp = TempDir::new().unwrap();
    let embedding_path = tmp.path().join("faces.json");
    let summary = summary_with_embeddings("input", vec![vec![1.0, 0.0, 0.5], vec![0.1, 0.2, 0.3]]);
    std::fs::write(
        &embedding_path,
        serde_json::to_string_pretty(&summary).unwrap(),
    )
    .unwrap();

    let store = StatefulStore::default();
    let keys = RecordingKeyProvider::default();
    let resolver = FixedResolver(tmp.path().to_path_buf());

    let enroll_config = FaceEnrollmentConfig {
        user: "alice".into(),
        embedding: embedding_path.clone(),
        store_dir: Some(tmp.path().to_path_buf()),
    };

    let outcome =
        run_face_enrollment_with(&enroll_config, &store, &keys, &resolver).expect("enroll works");
    assert_eq!(outcome.added.len(), 2);
    assert_eq!(store.embeddings.borrow().len(), 2);

    let remove_config = FaceRemovalConfig {
        user: "alice".into(),
        embedding_ids: vec![store.embeddings.borrow()[0].id.clone()],
        remove_all: false,
        store_dir: Some(tmp.path().to_path_buf()),
    };
    let removal =
        run_face_removal_with(&remove_config, &store, &keys, &resolver).expect("remove works");
    assert_eq!(removal.removed_ids.len(), 1);
    assert_eq!(removal.remaining, 1);
}

#[derive(Default)]
struct StatefulStore {
    embeddings: RefCell<Vec<EnrolledEmbedding>>,
}

impl FaceStore for StatefulStore {
    fn load(&self, _path: &Path, _key: Option<&[u8]>) -> AppResult<Vec<EnrolledEmbedding>> {
        Ok(self.embeddings.borrow().clone())
    }

    fn save(
        &self,
        _path: &Path,
        embeddings: &[EnrolledEmbedding],
        _key: Option<&[u8]>,
    ) -> AppResult<()> {
        *self.embeddings.borrow_mut() = embeddings.to_vec();
        Ok(())
    }

    fn delete(&self, _path: &Path) -> AppResult<()> {
        self.embeddings.borrow_mut().clear();
        Ok(())
    }
}

#[derive(Clone)]
struct FixedResolver(PathBuf);

impl StoreDirResolver for FixedResolver {
    fn resolve(&self, _override_dir: Option<&Path>) -> PathBuf {
        self.0.clone()
    }
}

#[derive(Default)]
struct RecordingKeyProvider {
    stored: RefCell<Vec<Vec<u8>>>,
}

impl KeyProvider for RecordingKeyProvider {
    fn fetch(&self, _user: &str) -> AppResult<EmbeddingKeyStatus> {
        Ok(EmbeddingKeyStatus::Missing)
    }

    fn store(&self, _user: &str, key: &[u8]) -> AppResult<()> {
        self.stored.borrow_mut().push(key.to_vec());
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
