use std::cmp::Ordering;
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use dlib_face_recognition::{
    FaceDetector, FaceDetectorTrait, FaceEncoderNetwork, FaceEncoderTrait, ImageMatrix,
    LandmarkPredictor, LandmarkPredictorTrait,
};
use image::{self, RgbImage};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tracing::debug;
use uuid::Uuid;

use crate::errors::{AppError, AppResult};

const LANDMARK_ENV: &str = "DLIB_LANDMARK_MODEL";
const ENCODER_ENV: &str = "DLIB_ENCODER_MODEL";
const DEFAULT_STORE_DIR: &str = "/var/lib/chissu-pam/models";
const FEATURE_STORE_ENV: &str = "CHISSU_PAM_STORE_DIR";

#[derive(Debug, Clone)]
pub struct FaceExtractionConfig {
    pub image: PathBuf,
    pub landmark_model: Option<PathBuf>,
    pub encoder_model: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub jitters: u32,
}

#[derive(Debug, Clone)]
pub struct FaceComparisonConfig {
    pub input: PathBuf,
    pub compare_targets: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct FaceEnrollmentConfig {
    pub user: String,
    pub descriptor: PathBuf,
    pub store_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct FaceRemovalConfig {
    pub user: String,
    pub descriptor_ids: Vec<String>,
    pub remove_all: bool,
    pub store_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct FaceModelPaths {
    pub landmark: PathBuf,
    pub encoder: PathBuf,
}

impl FaceExtractionConfig {
    pub fn resolve_models(&self) -> AppResult<FaceModelPaths> {
        let landmark = self
            .landmark_model
            .clone()
            .or_else(|| env::var(LANDMARK_ENV).ok().map(PathBuf::from))
            .ok_or(AppError::MissingModel {
                kind: "landmark predictor",
                flag: "--landmark-model",
                env: LANDMARK_ENV,
            })?;

        let encoder = self
            .encoder_model
            .clone()
            .or_else(|| env::var(ENCODER_ENV).ok().map(PathBuf::from))
            .ok_or(AppError::MissingModel {
                kind: "face encoding network",
                flag: "--encoder-model",
                env: ENCODER_ENV,
            })?;

        Ok(FaceModelPaths { landmark, encoder })
    }

    fn default_output_path(&self) -> PathBuf {
        let filename = format!(
            "face-features-{}.json",
            Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
        );
        PathBuf::from("captures").join("features").join(filename)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoundingBox {
    pub left: i64,
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FaceDescriptorRecord {
    pub bounding_box: BoundingBox,
    pub descriptor: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceExtractionSummary {
    pub success: bool,
    pub image_path: String,
    pub output_path: String,
    pub num_faces: usize,
    pub faces: Vec<FaceDescriptorRecord>,
    pub landmark_model: String,
    pub encoder_model: String,
    pub num_jitters: u32,
}

#[derive(Debug)]
pub struct FaceExtractionOutcome {
    pub summary: FaceExtractionSummary,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FaceComparisonScore {
    pub target_path: String,
    pub best_similarity: f64,
    pub input_face_index: usize,
    pub target_face_index: usize,
}

#[derive(Debug)]
pub struct FaceComparisonOutcome {
    pub scores: Vec<FaceComparisonScore>,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnrolledDescriptor {
    pub id: String,
    pub descriptor: Vec<f64>,
    pub bounding_box: BoundingBox,
    pub source: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EnrollmentRecord {
    pub id: String,
    pub descriptor_len: usize,
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

pub fn run_face_extraction(config: &FaceExtractionConfig) -> AppResult<FaceExtractionOutcome> {
    let models = config.resolve_models()?;
    let backend = DlibBackend::new(&models)?;
    run_face_extraction_with_backend(config, &models, &backend)
}

pub fn run_face_extraction_with_backend<B: FaceEmbeddingBackend>(
    config: &FaceExtractionConfig,
    models: &FaceModelPaths,
    backend: &B,
) -> AppResult<FaceExtractionOutcome> {
    let mut logs = Vec::new();

    let image_path = &config.image;
    if !image_path.exists() {
        return Err(AppError::MissingInput {
            path: image_path.clone(),
        });
    }

    let image = image::open(image_path).map_err(|source| AppError::ImageDecode {
        path: image_path.clone(),
        source,
    })?;
    let rgb: RgbImage = image.to_rgb8();
    logs.push(format!(
        "Loaded image {} ({}x{})",
        image_path.display(),
        rgb.width(),
        rgb.height()
    ));

    let faces = backend.extract(&rgb, config.jitters)?;
    logs.push(format!("Detected {} face(s)", faces.len()));
    if let Some(first) = faces.first() {
        logs.push(format!(
            "Descriptor vector length: {}",
            first.descriptor.len()
        ));
    }

    let output_path = config
        .output
        .clone()
        .unwrap_or_else(|| config.default_output_path());

    let summary = FaceExtractionSummary {
        success: true,
        image_path: image_path.display().to_string(),
        output_path: output_path.display().to_string(),
        num_faces: faces.len(),
        faces,
        landmark_model: models.landmark.display().to_string(),
        encoder_model: models.encoder.display().to_string(),
        num_jitters: config.jitters,
    };

    persist_summary(&summary, &output_path)?;
    logs.push(format!(
        "Saved descriptor data to {}",
        output_path.display()
    ));

    Ok(FaceExtractionOutcome { summary, logs })
}

fn persist_summary(summary: &FaceExtractionSummary, output_path: &Path) -> AppResult<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| AppError::FeatureWrite {
            path: parent.to_path_buf(),
            source: err,
        })?;
    }

    let file = File::create(output_path).map_err(|err| AppError::FeatureWrite {
        path: output_path.to_path_buf(),
        source: err,
    })?;

    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &summary)?;
    writer.flush().map_err(|err| AppError::FeatureWrite {
        path: output_path.to_path_buf(),
        source: err,
    })?;

    Ok(())
}

pub fn run_face_comparison(config: &FaceComparisonConfig) -> AppResult<FaceComparisonOutcome> {
    let input_summary = load_summary(&config.input)?;
    let input_dim = ensure_valid_faces(&input_summary.faces, &config.input)?;

    let mut scores = Vec::with_capacity(config.compare_targets.len());
    for target in &config.compare_targets {
        let summary = load_summary(target)?;
        let target_dim = ensure_valid_faces(&summary.faces, target)?;
        if target_dim != input_dim {
            return Err(AppError::InvalidFeatureFile {
                path: target.to_path_buf(),
                message: format!(
                    "descriptor length mismatch: expected {input_dim} values, found {target_dim}"
                ),
            });
        }

        let (best_similarity, input_index, target_index) =
            compute_best_similarity(&input_summary.faces, &summary.faces);
        scores.push(FaceComparisonScore {
            target_path: target.display().to_string(),
            best_similarity,
            input_face_index: input_index,
            target_face_index: target_index,
        });
    }

    scores.sort_by(|a, b| {
        b.best_similarity
            .partial_cmp(&a.best_similarity)
            .unwrap_or(Ordering::Equal)
    });

    let mut logs = Vec::new();
    logs.push(format!(
        "Loaded {} face(s) from {}",
        input_summary.faces.len(),
        config.input.display()
    ));
    logs.push("Similarity metric: cosine".to_string());
    for score in &scores {
        logs.push(format!(
            "Target {} => cosine similarity {:.4} (input face #{}, target face #{})",
            score.target_path,
            score.best_similarity,
            score.input_face_index,
            score.target_face_index
        ));
    }

    Ok(FaceComparisonOutcome { scores, logs })
}

pub fn run_face_enrollment(config: &FaceEnrollmentConfig) -> AppResult<FaceEnrollmentOutcome> {
    validate_user_name(&config.user)?;

    let mut logs = Vec::new();
    logs.push(format!(
        "Loading descriptor payload from {}",
        config.descriptor.display()
    ));

    let summary = load_summary(&config.descriptor)
        .map_err(|err| map_to_descriptor_validation(&config.descriptor, err))?;
    let descriptor_len = ensure_valid_faces(&summary.faces, &config.descriptor)
        .map_err(|err| map_to_descriptor_validation(&config.descriptor, err))?;
    logs.push(format!(
        "Validated {} descriptor(s) with length {}",
        summary.faces.len(),
        descriptor_len
    ));

    let store_path = user_store_path(config.store_dir.as_deref(), &config.user);
    let mut existing = read_enrolled_store(&store_path)?;

    if let Some(current_len) = existing.first().map(|entry| entry.descriptor.len()) {
        if current_len != descriptor_len {
            return Err(AppError::DescriptorValidation {
                path: store_path.clone(),
                message: format!(
                    "descriptor length mismatch with existing store (expected {current_len}, found {descriptor_len})"
                ),
            });
        }
    }

    let mut added = Vec::with_capacity(summary.faces.len());
    for face in &summary.faces {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let record = EnrolledDescriptor {
            id: id.clone(),
            descriptor: face.descriptor.clone(),
            bounding_box: face.bounding_box.clone(),
            source: config.descriptor.display().to_string(),
            created_at: created_at.clone(),
        };
        existing.push(record);
        added.push(EnrollmentRecord {
            id,
            descriptor_len,
            source: config.descriptor.display().to_string(),
            created_at,
        });
    }

    write_enrolled_store(&store_path, &existing)?;
    logs.push(format!(
        "Enrolled {} descriptor(s) for user {}",
        added.len(),
        config.user
    ));
    logs.push(format!("Feature store: {}", store_path.display()));

    Ok(FaceEnrollmentOutcome {
        user: config.user.clone(),
        store_path,
        added,
        logs,
    })
}

pub fn run_face_removal(config: &FaceRemovalConfig) -> AppResult<FaceRemovalOutcome> {
    validate_user_name(&config.user)?;

    let mut logs = Vec::new();
    let store_path = user_store_path(config.store_dir.as_deref(), &config.user);
    let existing = read_enrolled_store(&store_path)?;
    logs.push(format!(
        "Loaded {} descriptor(s) for user {}",
        existing.len(),
        config.user
    ));

    if config.remove_all {
        let removed_ids = existing
            .iter()
            .map(|entry| entry.id.clone())
            .collect::<Vec<_>>();
        if store_path.exists() {
            fs::remove_file(&store_path).map_err(|source| AppError::FeatureWrite {
                path: store_path.clone(),
                source,
            })?;
        }
        logs.push(format!("Removed all descriptors for user {}", config.user));
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
        if let Some(first) = config.descriptor_ids.first() {
            return Err(AppError::DescriptorNotFound {
                user: config.user.clone(),
                descriptor_id: first.clone(),
            });
        }
    }

    let requested: HashSet<String> = config.descriptor_ids.iter().cloned().collect();
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
            return Err(AppError::DescriptorNotFound {
                user: config.user.clone(),
                descriptor_id: missing.clone(),
            });
        }
    }

    write_enrolled_store(&store_path, &retained)?;
    logs.push(format!(
        "Removed {} descriptor(s) for user {}",
        removed_ids.len(),
        config.user
    ));
    logs.push(format!(
        "Feature store now contains {} descriptor(s)",
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

pub fn load_summary(path: &Path) -> AppResult<FaceExtractionSummary> {
    let file = File::open(path).map_err(|source| AppError::FeatureRead {
        path: path.to_path_buf(),
        source,
    })?;
    let reader = BufReader::new(file);
    let summary: FaceExtractionSummary = serde_json::from_reader(reader)?;
    Ok(summary)
}

pub fn write_enrolled_store(path: &Path, descriptors: &[EnrolledDescriptor]) -> AppResult<()> {
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
            serde_json::to_writer_pretty(&mut writer, descriptors)?;
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

pub fn read_enrolled_store(path: &Path) -> AppResult<Vec<EnrolledDescriptor>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path).map_err(|source| AppError::FeatureRead {
        path: path.to_path_buf(),
        source,
    })?;
    let reader = BufReader::new(file);
    let descriptors: Vec<EnrolledDescriptor> =
        serde_json::from_reader(reader).map_err(|err| AppError::InvalidFeatureFile {
            path: path.to_path_buf(),
            message: format!("invalid feature store contents: {err}"),
        })?;
    Ok(descriptors)
}

pub fn user_store_path(store_dir: Option<&Path>, user: &str) -> PathBuf {
    let base = if let Some(dir) = store_dir {
        dir.to_path_buf()
    } else if let Ok(env_value) = env::var(FEATURE_STORE_ENV) {
        PathBuf::from(env_value)
    } else {
        PathBuf::from(DEFAULT_STORE_DIR)
    };
    base.join(format!("{user}.json"))
}

pub fn load_enrolled_descriptors(
    store_dir: Option<&Path>,
    user: &str,
) -> AppResult<Vec<EnrolledDescriptor>> {
    let path = user_store_path(store_dir, user);
    read_enrolled_store(&path)
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

pub fn map_to_descriptor_validation(path: &Path, err: AppError) -> AppError {
    match err {
        AppError::InvalidFeatureFile { message, .. } => AppError::DescriptorValidation {
            path: path.to_path_buf(),
            message,
        },
        AppError::Serialization(source) => AppError::DescriptorValidation {
            path: path.to_path_buf(),
            message: source.to_string(),
        },
        other => other,
    }
}

pub fn ensure_valid_faces(faces: &[FaceDescriptorRecord], path: &Path) -> AppResult<usize> {
    if faces.is_empty() {
        return Err(AppError::InvalidFeatureFile {
            path: path.to_path_buf(),
            message: "contains no face descriptors".into(),
        });
    }

    let expected_len = faces[0].descriptor.len();
    if expected_len == 0 {
        return Err(AppError::InvalidFeatureFile {
            path: path.to_path_buf(),
            message: "descriptor vectors are empty".into(),
        });
    }

    for (idx, face) in faces.iter().enumerate() {
        if face.descriptor.len() != expected_len {
            return Err(AppError::InvalidFeatureFile {
                path: path.to_path_buf(),
                message: format!(
                    "descriptor length mismatch at face index {} (expected {}, found {})",
                    idx,
                    expected_len,
                    face.descriptor.len()
                ),
            });
        }

        let magnitude = face
            .descriptor
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        if magnitude <= f64::EPSILON {
            return Err(AppError::InvalidFeatureFile {
                path: path.to_path_buf(),
                message: format!("face index {idx} has zero-magnitude descriptor"),
            });
        }
    }

    Ok(expected_len)
}

pub fn compute_best_similarity(
    input_faces: &[FaceDescriptorRecord],
    target_faces: &[FaceDescriptorRecord],
) -> (f64, usize, usize) {
    let mut best_similarity = f64::NEG_INFINITY;
    let mut best_pair = (0, 0);

    for (i, input_face) in input_faces.iter().enumerate() {
        for (j, target_face) in target_faces.iter().enumerate() {
            let similarity = cosine_similarity(&input_face.descriptor, &target_face.descriptor);
            if similarity > best_similarity {
                best_similarity = similarity;
                best_pair = (i, j);
            }
        }
    }

    (best_similarity, best_pair.0, best_pair.1)
}

pub fn cosine_similarity(lhs: &[f64], rhs: &[f64]) -> f64 {
    let mut dot = 0.0;
    let mut norm_lhs = 0.0;
    let mut norm_rhs = 0.0;

    for (l, r) in lhs.iter().zip(rhs.iter()) {
        dot += l * r;
        norm_lhs += l * l;
        norm_rhs += r * r;
    }

    dot / (norm_lhs.sqrt() * norm_rhs.sqrt())
}

pub trait FaceEmbeddingBackend {
    fn extract(&self, image: &RgbImage, num_jitters: u32) -> AppResult<Vec<FaceDescriptorRecord>>;
}

pub struct DlibBackend {
    detector: FaceDetector,
    predictor: LandmarkPredictor,
    encoder: FaceEncoderNetwork,
}

impl DlibBackend {
    pub fn new(models: &FaceModelPaths) -> AppResult<Self> {
        debug!(path = %models.landmark.display(), "loading landmark model");
        let predictor =
            LandmarkPredictor::open(&models.landmark).map_err(|message| AppError::ModelLoad {
                path: models.landmark.clone(),
                message,
            })?;
        debug!(path = %models.encoder.display(), "loading encoder model");
        let encoder =
            FaceEncoderNetwork::open(&models.encoder).map_err(|message| AppError::ModelLoad {
                path: models.encoder.clone(),
                message,
            })?;
        let detector = FaceDetector::new();

        Ok(Self {
            detector,
            predictor,
            encoder,
        })
    }
}

impl FaceEmbeddingBackend for DlibBackend {
    fn extract(&self, image: &RgbImage, num_jitters: u32) -> AppResult<Vec<FaceDescriptorRecord>> {
        let matrix = ImageMatrix::from_image(image);
        let locations = self.detector.face_locations(&matrix);

        let mut landmarks = Vec::with_capacity(locations.len());
        for rect in locations.iter() {
            landmarks.push(self.predictor.face_landmarks(&matrix, rect));
        }

        let encodings = self
            .encoder
            .get_face_encodings(&matrix, &landmarks, num_jitters);

        let mut records = Vec::with_capacity(locations.len());
        for (rect, encoding) in locations.iter().zip(encodings.iter()) {
            let descriptor = encoding.as_ref().to_vec();
            let bounding_box = BoundingBox {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            };
            records.push(FaceDescriptorRecord {
                bounding_box,
                descriptor,
            });
        }

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;
    use serde_json::Value;
    use std::env;
    use std::fs::File;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn env_guard() -> &'static Mutex<()> {
        static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        GUARD.get_or_init(|| Mutex::new(()))
    }

    struct StubBackend {
        faces: Vec<FaceDescriptorRecord>,
    }

    impl FaceEmbeddingBackend for StubBackend {
        fn extract(
            &self,
            _image: &RgbImage,
            _num_jitters: u32,
        ) -> AppResult<Vec<FaceDescriptorRecord>> {
            Ok(self.faces.clone())
        }
    }

    fn stub_models() -> FaceModelPaths {
        FaceModelPaths {
            landmark: PathBuf::from("landmark.dat"),
            encoder: PathBuf::from("encoder.dat"),
        }
    }

    fn write_summary(path: &Path, summary: &FaceExtractionSummary) {
        let file = File::create(path).unwrap();
        serde_json::to_writer_pretty(file, summary).unwrap();
    }

    fn summary_with_descriptors(label: &str, descriptors: Vec<Vec<f64>>) -> FaceExtractionSummary {
        let faces = descriptors
            .into_iter()
            .enumerate()
            .map(|(idx, descriptor)| FaceDescriptorRecord {
                bounding_box: BoundingBox {
                    left: idx as i64,
                    top: 0,
                    right: 1,
                    bottom: 1,
                },
                descriptor,
            })
            .collect::<Vec<_>>();

        FaceExtractionSummary {
            success: true,
            image_path: format!("{}.png", label),
            output_path: format!("{}.json", label),
            num_faces: faces.len(),
            faces,
            landmark_model: "landmark.dat".into(),
            encoder_model: "encoder.dat".into(),
            num_jitters: 1,
        }
    }

    #[test]
    fn persists_descriptors_to_requested_output() {
        let tmp = TempDir::new().unwrap();
        let output_path = tmp.path().join("out.json");
        let image_path = tmp.path().join("input.png");

        let rgb = RgbImage::from_pixel(2, 2, Rgb([0, 0, 0]));
        rgb.save(&image_path).unwrap();

        let config = FaceExtractionConfig {
            image: image_path.clone(),
            landmark_model: Some(PathBuf::from("landmark.dat")),
            encoder_model: Some(PathBuf::from("encoder.dat")),
            output: Some(output_path.clone()),
            jitters: 1,
        };

        let descriptor = FaceDescriptorRecord {
            bounding_box: BoundingBox {
                left: 1,
                top: 2,
                right: 3,
                bottom: 4,
            },
            descriptor: vec![0.1, 0.2, 0.3],
        };

        let backend = StubBackend {
            faces: vec![descriptor.clone()],
        };

        let models = stub_models();

        let outcome = run_face_extraction_with_backend(&config, &models, &backend).unwrap();
        assert_eq!(outcome.summary.num_faces, 1);
        assert_eq!(outcome.summary.faces, vec![descriptor.clone()]);
        assert!(output_path.exists());

        let written = std::fs::read_to_string(output_path).unwrap();
        let json: Value = serde_json::from_str(&written).unwrap();
        assert_eq!(json["num_faces"], 1);
        assert_eq!(json["faces"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn missing_input_image_returns_error() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("missing.png");

        let config = FaceExtractionConfig {
            image: missing.clone(),
            landmark_model: Some(PathBuf::from("landmark.dat")),
            encoder_model: Some(PathBuf::from("encoder.dat")),
            output: None,
            jitters: 1,
        };

        let backend = StubBackend { faces: vec![] };
        let models = stub_models();

        let err = run_face_extraction_with_backend(&config, &models, &backend).unwrap_err();
        match err {
            AppError::MissingInput { path } => assert_eq!(path, missing),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn compare_reports_scores_sorted_by_similarity() {
        let tmp = TempDir::new().unwrap();
        let input_path = tmp.path().join("input.json");
        let target_a = tmp.path().join("target-a.json");
        let target_b = tmp.path().join("target-b.json");

        let input_summary = summary_with_descriptors("input", vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
        write_summary(&input_path, &input_summary);

        let target_summary_a = summary_with_descriptors("a", vec![vec![1.0, 0.0]]);
        write_summary(&target_a, &target_summary_a);

        let target_summary_b = summary_with_descriptors("b", vec![vec![-1.0, 0.0]]);
        write_summary(&target_b, &target_summary_b);

        let config = FaceComparisonConfig {
            input: input_path.clone(),
            compare_targets: vec![target_b.clone(), target_a.clone()],
        };

        let outcome = run_face_comparison(&config).unwrap();
        assert_eq!(outcome.scores.len(), 2);
        assert!(outcome.scores[0].target_path.ends_with("target-a.json"));
        assert!((outcome.scores[0].best_similarity - 1.0).abs() < 1e-6);
        assert!(outcome.scores[1].target_path.ends_with("target-b.json"));
        assert!(outcome.scores[1].best_similarity.abs() < 1e-6);
    }

    #[test]
    fn compare_errors_when_target_missing() {
        let tmp = TempDir::new().unwrap();
        let input_path = tmp.path().join("input.json");
        let missing_target = tmp.path().join("missing.json");

        let input_summary = summary_with_descriptors("input", vec![vec![1.0, 0.0]]);
        write_summary(&input_path, &input_summary);

        let config = FaceComparisonConfig {
            input: input_path,
            compare_targets: vec![missing_target.clone()],
        };

        let err = run_face_comparison(&config).unwrap_err();
        match err {
            AppError::FeatureRead { path, .. } => assert_eq!(path, missing_target),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn enroll_creates_store_and_records_ids() {
        let tmp = TempDir::new().unwrap();
        let store_dir = tmp.path().join("store");
        std::fs::create_dir_all(&store_dir).unwrap();

        let descriptor_path = tmp.path().join("faces.json");
        let summary =
            summary_with_descriptors("input", vec![vec![1.0, 0.0, 0.5], vec![0.1, 0.2, 0.3]]);
        write_summary(&descriptor_path, &summary);
        let config = FaceEnrollmentConfig {
            user: "alice".into(),
            descriptor: descriptor_path.clone(),
            store_dir: Some(store_dir.clone()),
        };
        let outcome = run_face_enrollment(&config).unwrap();
        assert_eq!(outcome.added.len(), 2);
        let store_path = store_dir.join("alice.json");
        assert_eq!(outcome.store_path, store_path);
        let written = std::fs::read_to_string(store_path).unwrap();
        let records: Vec<EnrolledDescriptor> = serde_json::from_str(&written).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].descriptor.len(), 3);
    }

    #[test]
    fn enroll_rejects_invalid_user_name() {
        let tmp = TempDir::new().unwrap();
        let store_dir = tmp.path().join("store");
        std::fs::create_dir_all(&store_dir).unwrap();

        let descriptor_path = tmp.path().join("faces.json");
        let summary = summary_with_descriptors("input", vec![vec![1.0, 0.0, 0.5]]);
        write_summary(&descriptor_path, &summary);

        let config = FaceEnrollmentConfig {
            user: "alice/bad".into(),
            descriptor: descriptor_path.clone(),
            store_dir: Some(store_dir.clone()),
        };
        let err = run_face_enrollment(&config).unwrap_err();
        match err {
            AppError::InvalidUser { .. } => {}
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn enroll_rejects_empty_descriptor_payload() {
        let tmp = TempDir::new().unwrap();
        let store_dir = tmp.path().join("store");
        std::fs::create_dir_all(&store_dir).unwrap();

        let descriptor_path = tmp.path().join("faces.json");
        let summary = summary_with_descriptors("input", vec![]);
        write_summary(&descriptor_path, &summary);

        let config = FaceEnrollmentConfig {
            user: "alice".into(),
            descriptor: descriptor_path.clone(),
            store_dir: Some(store_dir.clone()),
        };
        let err = run_face_enrollment(&config).unwrap_err();
        match err {
            AppError::DescriptorValidation { .. } => {}
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn remove_descriptor_by_id_updates_store() {
        let tmp = TempDir::new().unwrap();
        let store_dir = tmp.path().join("store");
        std::fs::create_dir_all(&store_dir).unwrap();

        let descriptor_path = tmp.path().join("faces.json");
        let summary =
            summary_with_descriptors("input", vec![vec![1.0, 0.0, 0.5], vec![0.1, 0.2, 0.3]]);
        write_summary(&descriptor_path, &summary);

        let enroll_config = FaceEnrollmentConfig {
            user: "alice".into(),
            descriptor: descriptor_path.clone(),
            store_dir: Some(store_dir.clone()),
        };
        let outcome = run_face_enrollment(&enroll_config).unwrap();
        let target_id = outcome.added[0].id.clone();

        let remove_config = FaceRemovalConfig {
            user: "alice".into(),
            descriptor_ids: vec![target_id.clone()],
            remove_all: false,
            store_dir: Some(store_dir.clone()),
        };
        let removal = run_face_removal(&remove_config).unwrap();
        assert_eq!(removal.removed_ids, vec![target_id]);
        assert_eq!(removal.remaining, 1);

        let written = std::fs::read_to_string(store_dir.join("alice.json")).unwrap();
        let records: Vec<EnrolledDescriptor> = serde_json::from_str(&written).unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn remove_unknown_descriptor_returns_error() {
        let tmp = TempDir::new().unwrap();
        let store_dir = tmp.path().join("store");
        std::fs::create_dir_all(&store_dir).unwrap();
        let remove_config = FaceRemovalConfig {
            user: "alice".into(),
            descriptor_ids: vec!["missing".into()],
            remove_all: false,
            store_dir: Some(store_dir.clone()),
        };
        let err = run_face_removal(&remove_config).unwrap_err();
        match err {
            AppError::DescriptorNotFound { .. } => {}
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn remove_all_clears_store_file() {
        let tmp = TempDir::new().unwrap();
        let store_dir = tmp.path().join("store");
        std::fs::create_dir_all(&store_dir).unwrap();

        let descriptor_path = tmp.path().join("faces.json");
        let summary = summary_with_descriptors("input", vec![vec![1.0, 0.0, 0.5]]);
        write_summary(&descriptor_path, &summary);

        let enroll_config = FaceEnrollmentConfig {
            user: "alice".into(),
            descriptor: descriptor_path.clone(),
            store_dir: Some(store_dir.clone()),
        };
        run_face_enrollment(&enroll_config).unwrap();

        let remove_config = FaceRemovalConfig {
            user: "alice".into(),
            descriptor_ids: vec![],
            remove_all: true,
            store_dir: Some(store_dir.clone()),
        };
        let outcome = run_face_removal(&remove_config).unwrap();
        assert!(outcome.cleared);
        assert_eq!(outcome.remaining, 0);
        assert!(!store_dir.join("alice.json").exists());
    }

    #[test]
    fn user_store_path_prefers_env_when_no_override() {
        let _lock = env_guard().lock().unwrap();
        let tmp = TempDir::new().unwrap();
        let env_dir = tmp.path().join("env-store");
        std::fs::create_dir_all(&env_dir).unwrap();
        env::set_var(FEATURE_STORE_ENV, env_dir.to_str().unwrap());

        let resolved = user_store_path(None, "alice");
        assert_eq!(resolved, env_dir.join("alice.json"));

        env::remove_var(FEATURE_STORE_ENV);
    }

    #[test]
    fn user_store_path_defaults_to_builtin_directory() {
        let _lock = env_guard().lock().unwrap();
        env::remove_var(FEATURE_STORE_ENV);
        let resolved = user_store_path(None, "bob");
        assert_eq!(resolved, PathBuf::from(DEFAULT_STORE_DIR).join("bob.json"));
    }
}
