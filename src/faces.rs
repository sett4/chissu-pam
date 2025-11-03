use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use dlib_face_recognition::{
    FaceDetector, FaceDetectorTrait, FaceEncoderNetwork, FaceEncoderTrait, ImageMatrix,
    LandmarkPredictor, LandmarkPredictorTrait,
};
use image::{self, RgbImage};
use serde::Serialize;
use tracing::debug;

use crate::cli::FaceExtractArgs;
use crate::errors::{AppError, AppResult};

const LANDMARK_ENV: &str = "DLIB_LANDMARK_MODEL";
const ENCODER_ENV: &str = "DLIB_ENCODER_MODEL";

#[derive(Debug, Clone)]
pub struct FaceExtractionConfig {
    pub image: PathBuf,
    pub landmark_model: Option<PathBuf>,
    pub encoder_model: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub jitters: u32,
}

impl From<&FaceExtractArgs> for FaceExtractionConfig {
    fn from(args: &FaceExtractArgs) -> Self {
        Self {
            image: args.image.clone(),
            landmark_model: args.landmark_model.clone(),
            encoder_model: args.encoder_model.clone(),
            output: args.output.clone(),
            jitters: args.jitters,
        }
    }
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

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoundingBox {
    pub left: i64,
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FaceDescriptorRecord {
    pub bounding_box: BoundingBox,
    pub descriptor: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
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

pub fn run_face_extraction(config: &FaceExtractionConfig) -> AppResult<FaceExtractionOutcome> {
    let models = config.resolve_models()?;
    let backend = DlibBackend::new(&models)?;
    run_face_extraction_with_backend(config, &models, &backend)
}

fn run_face_extraction_with_backend<B: FaceEmbeddingBackend>(
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

trait FaceEmbeddingBackend {
    fn extract(&self, image: &RgbImage, num_jitters: u32) -> AppResult<Vec<FaceDescriptorRecord>>;
}

struct DlibBackend {
    detector: FaceDetector,
    predictor: LandmarkPredictor,
    encoder: FaceEncoderNetwork,
}

impl DlibBackend {
    fn new(models: &FaceModelPaths) -> AppResult<Self> {
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
                left: rect.left as i64,
                top: rect.top as i64,
                right: rect.right as i64,
                bottom: rect.bottom as i64,
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
    use serde_json::Value;
    use tempfile::TempDir;
    use image::Rgb;

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
}
