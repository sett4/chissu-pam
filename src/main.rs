mod capture;
mod cli;
mod config;
mod errors;
mod faces;
mod output;

use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

use crate::capture::CaptureConfig;
use crate::cli::{Cli, Commands, FacesCommands, OutputMode};
use crate::config as config_loader;
use crate::errors::AppError;
use crate::faces::{
    FaceComparisonConfig, FaceEnrollmentConfig, FaceExtractionConfig, FaceRemovalConfig,
};
use crate::output::{
    render_error, render_face_compare, render_face_enroll, render_face_remove, render_face_success,
    render_success,
};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mode = cli.output_mode();
    init_tracing(cli.verbose, mode);

    match run(cli, mode) {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            render_error(&err, mode);
            err.exit_code()
        }
    }
}

fn run(cli: Cli, mode: OutputMode) -> Result<(), AppError> {
    match cli.command {
        Commands::Capture(args) => {
            let config = CaptureConfig::from(&args);
            let outcome = capture::run_capture(&config)?;
            render_success(&outcome, mode)?;
        }
        Commands::Faces(cmd) => match cmd {
            FacesCommands::Extract(args) => {
                let config = FaceExtractionConfig::from(&args);
                let outcome = faces::run_face_extraction(&config)?;
                render_face_success(&outcome, mode)?;
            }
            FacesCommands::Compare(args) => {
                let config = FaceComparisonConfig::from(&args);
                let outcome = faces::run_face_comparison(&config)?;
                render_face_compare(&outcome, mode)?;
            }
            FacesCommands::Enroll(mut args) => {
                args.store_dir = config_loader::resolve_store_dir(args.store_dir.take())?;
                let config = FaceEnrollmentConfig::from(&args);
                let outcome = faces::run_face_enrollment(&config)?;
                render_face_enroll(&outcome, mode)?;
            }
            FacesCommands::Remove(mut args) => {
                args.store_dir = config_loader::resolve_store_dir(args.store_dir.take())?;
                let config = FaceRemovalConfig::from(&args);
                let outcome = faces::run_face_removal(&config)?;
                render_face_remove(&outcome, mode)?;
            }
        },
    }
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
