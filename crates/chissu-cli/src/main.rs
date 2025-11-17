use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

use chissu_cli::cli::{Cli, Commands, OutputMode};
use chissu_cli::commands::CommandHandler;
use chissu_cli::errors::AppResult;
use chissu_cli::output::render_error;

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

fn run(command: Commands, mode: OutputMode, verbose: bool) -> AppResult<ExitCode> {
    let handler: Box<dyn CommandHandler> = command.into();
    handler.execute(mode, verbose)
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
