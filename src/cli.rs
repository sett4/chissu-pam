use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "study-rust-v4l2",
    about = "Capture infrared frames from V4L2 webcams",
    version
)]
pub struct Cli {
    /// Emit structured JSON to stdout instead of human-readable logs
    #[arg(long)]
    pub json: bool,

    /// Increase verbosity (may be used multiple times)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Capture a single frame from an infrared-capable webcam
    Capture(CaptureArgs),
}

#[derive(Debug, Args)]
pub struct CaptureArgs {
    /// Video device path (e.g. /dev/video2) or index (e.g. 0)
    #[arg(long)]
    pub device: Option<String>,

    /// Requested pixel format FourCC (e.g. Y16, GREY)
    #[arg(long, default_value = "Y16")]
    pub pixel_format: String,

    /// Requested frame width
    #[arg(long)]
    pub width: Option<u32>,

    /// Requested frame height
    #[arg(long)]
    pub height: Option<u32>,

    /// Exposure absolute value (if supported by device)
    #[arg(long)]
    pub exposure: Option<i32>,

    /// Analog gain value (if supported by device)
    #[arg(long)]
    pub gain: Option<i32>,

    /// Enable device-provided automatic exposure before capture
    #[arg(long)]
    pub auto_exposure: bool,

    /// Enable device-provided automatic gain before capture
    #[arg(long)]
    pub auto_gain: bool,

    /// Optional output file path (defaults to captures/<timestamp>.png)
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Png,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputMode {
    Human,
    Json,
}

impl From<bool> for OutputMode {
    fn from(json: bool) -> Self {
        if json {
            OutputMode::Json
        } else {
            OutputMode::Human
        }
    }
}

impl Cli {
    pub fn output_mode(&self) -> OutputMode {
        OutputMode::from(self.json)
    }
}
