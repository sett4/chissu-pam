use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};

pub const DEFAULT_PIXEL_FORMAT: &str = "Y16";
pub const DEFAULT_WARMUP_FRAMES: u32 = 4;

#[derive(Debug, Parser)]
#[command(
    name = "chissu-pam",
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
    /// Operations that work with facial feature extraction pipelines
    #[command(subcommand)]
    Faces(FacesCommands),
}

#[derive(Debug, Subcommand)]
pub enum FacesCommands {
    /// Extract face descriptors from an existing PNG image
    Extract(FaceExtractArgs),
    /// Compare face descriptor files produced by the extract command
    Compare(FaceCompareArgs),
    /// Enroll descriptors into a per-user feature store
    Enroll(FaceEnrollArgs),
    /// Remove descriptors from a per-user feature store
    Remove(FaceRemoveArgs),
}

#[derive(Debug, Args)]
pub struct CaptureArgs {
    /// Video device path (e.g. /dev/video2) or index (e.g. 0)
    #[arg(long)]
    pub device: Option<String>,

    /// Requested pixel format FourCC (e.g. Y16, GREY). Defaults to config or `Y16`.
    #[arg(long)]
    pub pixel_format: Option<String>,

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

    /// Number of warm-up frames to discard before saving the captured image. Defaults to config or 4.
    #[arg(long)]
    pub warmup_frames: Option<u32>,

    /// Optional output file path (defaults to captures/<timestamp>.png)
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct FaceExtractArgs {
    /// Path to the PNG image that contains one or more faces
    pub image: PathBuf,

    /// Optional path to the dlib landmark predictor model (falls back to $DLIB_LANDMARK_MODEL)
    #[arg(long)]
    pub landmark_model: Option<PathBuf>,

    /// Optional path to the dlib face recognition network (falls back to $DLIB_ENCODER_MODEL)
    #[arg(long)]
    pub encoder_model: Option<PathBuf>,

    /// Optional output file for serialized descriptors (defaults to captures/features/<timestamp>.json)
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Number of image jitters to run before encoding (controls descriptor stability)
    #[arg(long, default_value_t = 1)]
    pub jitters: u32,
}

#[derive(Debug, Args)]
pub struct FaceCompareArgs {
    /// Path to the descriptor JSON exported by `faces extract`
    #[arg(long)]
    pub input: PathBuf,

    /// Descriptor JSON paths to compare against the input (repeatable)
    #[arg(long = "compare-target", required = true)]
    pub compare_targets: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub struct FaceEnrollArgs {
    /// Target operating system user name
    #[arg(long)]
    pub user: String,

    /// Path to the descriptor JSON exported by `faces extract`
    pub descriptor: PathBuf,

    /// Optional directory that stores enrolled descriptors (overrides config/env defaults)
    #[arg(long)]
    pub store_dir: Option<PathBuf>,
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("selector")
        .required(true)
        .args(["descriptor_id", "all"]),
))]
pub struct FaceRemoveArgs {
    /// Target operating system user name
    #[arg(long)]
    pub user: String,

    /// Descriptor identifier to remove (repeat flag to delete multiple)
    #[arg(long, conflicts_with = "all")]
    pub descriptor_id: Vec<String>,

    /// Remove all descriptors for the user
    #[arg(long)]
    pub all: bool,

    /// Optional directory that stores enrolled descriptors (overrides config/env defaults)
    #[arg(long)]
    pub store_dir: Option<PathBuf>,
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
