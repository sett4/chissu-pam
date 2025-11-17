use std::cmp::Ordering;
use std::path::PathBuf;

use serde::Serialize;

use crate::errors::{AppError, AppResult};
use crate::faces::extractor::{ensure_valid_faces, load_summary, FaceEmbeddingRecord};

#[derive(Debug, Clone)]
pub struct FaceComparisonConfig {
    pub input: PathBuf,
    pub compare_targets: Vec<PathBuf>,
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
                    "embedding length mismatch: expected {input_dim} values, found {target_dim}"
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

pub fn compute_best_similarity(
    input_faces: &[FaceEmbeddingRecord],
    target_faces: &[FaceEmbeddingRecord],
) -> (f64, usize, usize) {
    let mut best_similarity = f64::NEG_INFINITY;
    let mut best_pair = (0, 0);

    for (i, input_face) in input_faces.iter().enumerate() {
        for (j, target_face) in target_faces.iter().enumerate() {
            let similarity = cosine_similarity(&input_face.embedding, &target_face.embedding);
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    use crate::faces::extractor::{BoundingBox, FaceExtractionSummary};

    #[test]
    fn compare_reports_scores_sorted_by_similarity() {
        let tmp = TempDir::new().unwrap();
        let input_path = tmp.path().join("input.json");
        let target_a = tmp.path().join("target-a.json");
        let target_b = tmp.path().join("target-b.json");

        let input_summary = summary_with_embeddings("input", vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
        write_summary(&input_path, &input_summary);

        let target_summary_a = summary_with_embeddings("a", vec![vec![1.0, 0.0]]);
        write_summary(&target_a, &target_summary_a);

        let target_summary_b = summary_with_embeddings("b", vec![vec![-1.0, 0.0]]);
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

        let input_summary = summary_with_embeddings("input", vec![vec![1.0, 0.0]]);
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

    fn write_summary(path: &PathBuf, summary: &FaceExtractionSummary) {
        std::fs::write(path, serde_json::to_string_pretty(summary).unwrap()).unwrap();
    }
}
