use std::error::Error;
use std::io::{self, Write};

use serde_json::json;

use crate::capture::CaptureOutcome;
use crate::cli::OutputMode;
use crate::errors::{AppError, AppResult};
use crate::faces::FaceExtractionOutcome;

pub fn render_success(outcome: &CaptureOutcome, mode: OutputMode) -> AppResult<()> {
    match mode {
        OutputMode::Human => {
            for line in &outcome.logs {
                println!("{}", line);
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
                println!("{}", line);
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

pub fn render_error(err: &AppError, mode: OutputMode) {
    match mode {
        OutputMode::Human => {
            eprintln!("error: {}", err.human_message());
            if let Some(source) = err.source() {
                eprintln!("cause: {}", source);
            }
        }
        OutputMode::Json => {
            let payload = json!({
                "success": false,
                "error": err.human_message(),
            });
            if let Ok(json) = serde_json::to_string(&payload) {
                println!("{}", json);
            }
            if let Some(source) = err.source() {
                eprintln!("cause: {}", source);
            }
        }
    }
}
