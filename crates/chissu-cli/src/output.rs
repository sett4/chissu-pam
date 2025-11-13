use std::error::Error;
use std::io::{self, Write};

use serde_json::json;

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
