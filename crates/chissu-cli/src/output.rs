use std::error::Error;
use std::io::{self, Write};

use serde_json::{json, Value};

use crate::auto_enroll::AutoEnrollOutcome;
use crate::capture::CaptureOutcome;
use crate::cli::OutputMode;
use crate::errors::{AppError, AppResult};
use crate::faces::{
    FaceComparisonOutcome, FaceEnrollmentOutcome, FaceExtractionOutcome, FaceRemovalOutcome,
};
use crate::keyring::KeyringCheckSummary;

pub fn render_success(outcome: &CaptureOutcome, mode: OutputMode) -> AppResult<()> {
    match mode {
        OutputMode::Human => {
            for line in &outcome.logs {
                println!("{line}");
            }
            println!("Capture successful: {}", outcome.summary.output_path);
        }
        OutputMode::Json => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let payload = serde_json::to_string(&outcome.summary)?;
            handle.write_all(payload.as_bytes())?;
            handle.write_all(b"\n")?;
        }
    }
    Ok(())
}

pub fn render_face_success(outcome: &FaceExtractionOutcome, mode: OutputMode) -> AppResult<()> {
    match mode {
        OutputMode::Human => {
            for line in &outcome.logs {
                println!("{line}");
            }
            println!(
                "Feature extraction successful: {} (faces: {})",
                outcome.summary.output_path, outcome.summary.num_faces
            );
        }
        OutputMode::Json => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let payload = serde_json::to_string(&outcome.summary)?;
            handle.write_all(payload.as_bytes())?;
            handle.write_all(b"\n")?;
        }
    }
    Ok(())
}

pub fn render_face_compare(outcome: &FaceComparisonOutcome, mode: OutputMode) -> AppResult<()> {
    match mode {
        OutputMode::Human => {
            for line in &outcome.logs {
                println!("{line}");
            }
        }
        OutputMode::Json => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let payload = serde_json::to_string(&outcome.scores)?;
            handle.write_all(payload.as_bytes())?;
            handle.write_all(b"\n")?;
        }
    }
    Ok(())
}

pub fn render_face_enroll(outcome: &FaceEnrollmentOutcome, mode: OutputMode) -> AppResult<()> {
    match mode {
        OutputMode::Human => {
            for line in &outcome.logs {
                println!("{line}");
            }
            println!(
                "Enrollment successful: {} descriptor(s) added to {}",
                outcome.added.len(),
                outcome.store_path.display()
            );
        }
        OutputMode::Json => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let payload = serde_json::to_string(&json!({
                "user": outcome.user,
                "store_path": outcome.store_path.display().to_string(),
                "added": outcome.added,
            }))?;
            handle.write_all(payload.as_bytes())?;
            handle.write_all(b"\n")?;
        }
    }
    Ok(())
}

pub fn render_auto_enroll(
    outcome: &AutoEnrollOutcome,
    mode: OutputMode,
    verbose: bool,
) -> AppResult<()> {
    match mode {
        OutputMode::Human => {
            if verbose {
                for line in &outcome.logs {
                    tracing::info!("{line}");
                }
            }
            println!(
                "Auto enrollment successful: {} descriptor(s) added for user {}",
                outcome.enrollment.added.len(),
                outcome.enrollment.user
            );
            println!(
                "Encrypted store: {}",
                outcome.enrollment.store_path.display()
            );
            if verbose {
                if outcome.capture_deleted {
                    tracing::info!(
                        "Captured image {} deleted after enrollment",
                        outcome.capture_path.display()
                    );
                }
                if outcome.descriptor_deleted {
                    tracing::info!(
                        "Descriptor payload {} deleted after enrollment",
                        outcome.descriptor_path.display()
                    );
                }
            }
        }
        OutputMode::Json => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let payload = auto_enroll_json_payload(outcome);
            let payload = serde_json::to_string(&payload)?;
            handle.write_all(payload.as_bytes())?;
            handle.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn auto_enroll_json_payload(outcome: &AutoEnrollOutcome) -> Value {
    let descriptor_ids: Vec<String> = outcome
        .enrollment
        .added
        .iter()
        .map(|record| record.id.clone())
        .collect();
    json!({
        "user": outcome.enrollment.user,
        "target_user": outcome.target_user,
        "store_path": outcome.enrollment.store_path.display().to_string(),
        "added": outcome.enrollment.added,
        "descriptor_ids": descriptor_ids,
        "captured_image": outcome.capture_path.display().to_string(),
        "captured_image_deleted": outcome.capture_deleted,
        "descriptor_file_deleted": outcome.descriptor_deleted,
        "faces_detected": outcome.faces_detected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faces::{EnrollmentRecord, FaceEnrollmentOutcome};
    use chrono::SecondsFormat;
    use std::path::PathBuf;

    #[test]
    fn auto_enroll_json_includes_required_fields() {
        let enrollment = FaceEnrollmentOutcome {
            user: "alice".into(),
            store_path: PathBuf::from("/var/lib/chissu-pam/models/alice.json"),
            added: vec![EnrollmentRecord {
                id: "abc".into(),
                descriptor_len: 128,
                source: "captures/tmp/features.json".into(),
                created_at: chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            }],
            logs: vec![],
        };

        let outcome = AutoEnrollOutcome {
            target_user: "alice".into(),
            capture_path: PathBuf::from("captures/auto-enroll/capture.png"),
            descriptor_path: PathBuf::from("captures/auto-enroll/features.json"),
            capture_deleted: true,
            descriptor_deleted: true,
            faces_detected: 1,
            enrollment,
            logs: vec![],
        };

        let payload = auto_enroll_json_payload(&outcome);
        assert_eq!(
            payload["captured_image"],
            "captures/auto-enroll/capture.png"
        );
        assert_eq!(payload["target_user"], "alice");
        assert_eq!(payload["descriptor_ids"].as_array().unwrap().len(), 1);
        assert_eq!(payload["captured_image_deleted"], true);
        assert_eq!(payload["descriptor_file_deleted"], true);
        assert_eq!(payload["faces_detected"], 1);
    }
}

pub fn render_face_remove(outcome: &FaceRemovalOutcome, mode: OutputMode) -> AppResult<()> {
    match mode {
        OutputMode::Human => {
            for line in &outcome.logs {
                println!("{line}");
            }
            println!(
                "Removal successful: removed {} descriptor(s); remaining {}",
                outcome.removed_ids.len(),
                outcome.remaining
            );
        }
        OutputMode::Json => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let payload = serde_json::to_string(&json!({
                "user": outcome.user,
                "store_path": outcome.store_path.display().to_string(),
                "removed_ids": outcome.removed_ids,
                "remaining": outcome.remaining,
                "cleared": outcome.cleared,
            }))?;
            handle.write_all(payload.as_bytes())?;
            handle.write_all(b"\n")?;
        }
    }
    Ok(())
}

pub fn render_error(err: &AppError, mode: OutputMode) {
    if let AppError::SecretServiceUnavailable {
        user,
        service,
        message,
    } = err
    {
        match mode {
            OutputMode::Human => {
                eprintln!(
                    "Secret Service unavailable for user '{user}' (service '{service}'): {message}"
                );
            }
            OutputMode::Json => {
                let payload = json!({
                    "status": "error",
                    "user": user,
                    "service": service,
                    "error": message,
                });
                println!("{payload}");
            }
        }
        return;
    }

    match mode {
        OutputMode::Human => {
            eprintln!("error: {}", err.human_message());
            if let Some(source) = err.source() {
                eprintln!("cause: {source}");
            }
        }
        OutputMode::Json => {
            let payload = json!({
                "success": false,
                "error": err.human_message(),
            });
            if let Ok(json) = serde_json::to_string(&payload) {
                println!("{json}");
            }
            if let Some(source) = err.source() {
                eprintln!("cause: {source}");
            }
        }
    }
}

pub fn render_keyring_check(summary: &KeyringCheckSummary, mode: OutputMode) -> AppResult<()> {
    match mode {
        OutputMode::Human => {
            println!(
                "Secret Service available for user '{}' (service '{}')",
                summary.user, summary.service
            );
        }
        OutputMode::Json => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let payload = serde_json::to_string(&json!({
                "status": "ok",
                "user": summary.user,
                "service": summary.service,
            }))?;
            handle.write_all(payload.as_bytes())?;
            handle.write_all(b"\n")?;
        }
    }
    Ok(())
}
