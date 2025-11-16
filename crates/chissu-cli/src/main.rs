mod auto_enroll;
mod capture;
mod cli;
mod config;
mod doctor;
mod errors;
mod faces;
mod keyring;
mod output;

use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

use crate::cli::{
    Cli, Commands, EnrollArgs, FacesCommands, KeyringCommands, OutputMode, DEFAULT_PIXEL_FORMAT,
    DEFAULT_WARMUP_FRAMES,
};
use crate::config as config_loader;
use crate::errors::AppError;
use crate::faces::{
    FaceComparisonConfig, FaceEnrollmentConfig, FaceExtractionConfig, FaceRemovalConfig,
};
use crate::output::{
    render_auto_enroll, render_doctor, render_error, render_face_compare, render_face_enroll,
    render_face_remove, render_face_success, render_success,
};

fn main() -> ExitCode {
    let Cli {
        json,
        verbose,
        command,
    } = Cli::parse();
    let mode = OutputMode::from(json);
    init_tracing(verbose, mode);

    match run(command, mode, verbose > 0) {
        Ok(code) => code,
        Err(err) => {
            render_error(&err, mode);
            err.exit_code()
        }
    }
}

fn run(command: Commands, mode: OutputMode, verbose: bool) -> Result<ExitCode, AppError> {
    match command {
        Commands::Capture(args) => {
            let capture_defaults = config_loader::load_capture_defaults()?;
            if args.device.is_none() && capture_defaults.device.is_none() {
                tracing::info!(
                    target: "capture.defaults",
                    "No --device flag or config video_device found; defaulting to /dev/video0"
                );
            }
            if args.pixel_format.is_none() && capture_defaults.pixel_format.is_none() {
                tracing::info!(
                    target: "capture.defaults",
                    "No --pixel-format flag or config pixel_format found; defaulting to {}",
                    DEFAULT_PIXEL_FORMAT
                );
            }
            if args.warmup_frames.is_none() && capture_defaults.warmup_frames.is_none() {
                tracing::info!(
                    target: "capture.defaults",
                    "No --warmup-frames flag or config warmup_frames found; defaulting to {}",
                    DEFAULT_WARMUP_FRAMES
                );
            }
            let config = capture::build_capture_config(&args, &capture_defaults);
            let outcome = capture::run_capture(&config)?;
            render_success(&outcome, mode)?;
            return Ok(ExitCode::SUCCESS);
        }
        Commands::Enroll(args) => run_enroll(args, mode, verbose)?,
        Commands::Faces(cmd) => match cmd {
            FacesCommands::Extract(args) => {
                let config = FaceExtractionConfig::from(&args);
                let outcome = faces::run_face_extraction(&config)?;
                render_face_success(&outcome, mode)?;
                return Ok(ExitCode::SUCCESS);
            }
            FacesCommands::Compare(args) => {
                let config = FaceComparisonConfig::from(&args);
                let outcome = faces::run_face_comparison(&config)?;
                render_face_compare(&outcome, mode)?;
                return Ok(ExitCode::SUCCESS);
            }
            FacesCommands::Enroll(mut args) => {
                args.store_dir = config_loader::resolve_store_dir(args.store_dir.take())?;
                let config = FaceEnrollmentConfig::from(&args);
                let outcome = faces::run_face_enrollment(&config)?;
                render_face_enroll(&outcome, mode)?;
                return Ok(ExitCode::SUCCESS);
            }
            FacesCommands::Remove(mut args) => {
                args.store_dir = config_loader::resolve_store_dir(args.store_dir.take())?;
                let config = FaceRemovalConfig::from(&args);
                let outcome = faces::run_face_removal(&config)?;
                render_face_remove(&outcome, mode)?;
                return Ok(ExitCode::SUCCESS);
            }
        },
        Commands::Keyring(cmd) => match cmd {
            KeyringCommands::Check(_) => {
                keyring::run_keyring_check(mode)?;
                return Ok(ExitCode::SUCCESS);
            }
        },
        Commands::Doctor => {
            let outcome = doctor::run_doctor()?;
            render_doctor(&outcome, mode)?;
            let exit = if outcome.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            };
            return Ok(exit);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run_enroll(args: EnrollArgs, mode: OutputMode, verbose: bool) -> Result<(), AppError> {
    let outcome = auto_enroll::run_auto_enroll(&args)?;
    render_auto_enroll(&outcome, mode, verbose)?;
    Ok(())
}

fn init_tracing(verbose: u8, _mode: OutputMode) {
    let level = match verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_writer(std::io::stderr)
        .with_filter(env_filter);

    let registry = tracing_subscriber::registry().with(fmt_layer);
    if tracing::subscriber::set_global_default(registry).is_err() {
        // Already initialised (tests).
    }
}
