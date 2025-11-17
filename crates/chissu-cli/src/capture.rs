pub use chissu_face_core::capture::*;

use std::any::Any;
use std::process::ExitCode;

use crate::cli::{CaptureArgs, OutputMode, DEFAULT_PIXEL_FORMAT, DEFAULT_WARMUP_FRAMES};
use crate::commands::CommandHandler;
use crate::config::{self, CaptureDefaults};
use crate::errors::AppResult;
use crate::output::render_success;

type CaptureDefaultsLoader = dyn Fn() -> AppResult<CaptureDefaults> + Send + Sync;
type CaptureRunner = dyn Fn(&CaptureConfig) -> AppResult<CaptureOutcome> + Send + Sync;
type CaptureRenderer = dyn Fn(&CaptureOutcome, OutputMode) -> AppResult<()> + Send + Sync;

pub fn build_capture_config(args: &CaptureArgs, defaults: &CaptureDefaults) -> CaptureConfig {
    let device = args.device.clone().or_else(|| defaults.device.clone());
    let pixel_format = args
        .pixel_format
        .clone()
        .or_else(|| defaults.pixel_format.clone())
        .unwrap_or_else(|| DEFAULT_PIXEL_FORMAT.to_string());
    let warmup_frames = args
        .warmup_frames
        .or(defaults.warmup_frames)
        .unwrap_or(DEFAULT_WARMUP_FRAMES);

    CaptureConfig {
        device: DeviceLocator::from_option(device),
        pixel_format,
        width: args.width,
        height: args.height,
        exposure: args.exposure,
        gain: args.gain,
        auto_exposure: args.auto_exposure,
        auto_gain: args.auto_gain,
        warmup_frames,
        output: args.output.clone(),
    }
}

pub struct CaptureHandler {
    args: CaptureArgs,
    load_defaults: Box<CaptureDefaultsLoader>,
    run_capture: Box<CaptureRunner>,
    render: Box<CaptureRenderer>,
}

impl CaptureHandler {
    pub fn new(args: CaptureArgs) -> Self {
        Self::with_dependencies(
            args,
            config::load_capture_defaults,
            run_capture,
            render_success,
        )
    }

    pub fn with_dependencies(
        args: CaptureArgs,
        load_defaults: impl Fn() -> AppResult<CaptureDefaults> + Send + Sync + 'static,
        run_capture: impl Fn(&CaptureConfig) -> AppResult<CaptureOutcome> + Send + Sync + 'static,
        render: impl Fn(&CaptureOutcome, OutputMode) -> AppResult<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            args,
            load_defaults: Box::new(load_defaults),
            run_capture: Box::new(run_capture),
            render: Box::new(render),
        }
    }
}

impl CommandHandler for CaptureHandler {
    fn execute(&self, mode: OutputMode, _verbose: bool) -> AppResult<ExitCode> {
        let defaults = (self.load_defaults)()?;
        log_capture_defaults(&self.args, &defaults);
        let config = build_capture_config(&self.args, &defaults);
        let outcome = (self.run_capture)(&config)?;
        (self.render)(&outcome, mode)?;
        Ok(ExitCode::SUCCESS)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn log_capture_defaults(args: &CaptureArgs, defaults: &CaptureDefaults) {
    if args.device.is_none() && defaults.device.is_none() {
        tracing::info!(
            target: "capture.defaults",
            "No --device flag or config video_device found; defaulting to /dev/video0"
        );
    }
    if args.pixel_format.is_none() && defaults.pixel_format.is_none() {
        tracing::info!(
            target: "capture.defaults",
            "No --pixel-format flag or config pixel_format found; defaulting to {}",
            DEFAULT_PIXEL_FORMAT
        );
    }
    if args.warmup_frames.is_none() && defaults.warmup_frames.is_none() {
        tracing::info!(
            target: "capture.defaults",
            "No --warmup-frames flag or config warmup_frames found; defaulting to {}",
            DEFAULT_WARMUP_FRAMES
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn base_args() -> CaptureArgs {
        CaptureArgs {
            device: None,
            pixel_format: None,
            width: None,
            height: None,
            exposure: None,
            gain: None,
            auto_exposure: false,
            auto_gain: false,
            warmup_frames: None,
            output: None,
        }
    }

    fn defaults(device: Option<&str>, pixel: Option<&str>, warmup: Option<u32>) -> CaptureDefaults {
        CaptureDefaults {
            device: device.map(|d| d.to_string()),
            pixel_format: pixel.map(|p| p.to_string()),
            warmup_frames: warmup,
        }
    }

    #[test]
    fn cli_values_override_config_defaults() {
        let mut args = base_args();
        args.device = Some("/dev/video9".into());
        args.pixel_format = Some("GREY".into());
        args.warmup_frames = Some(7);

        let config =
            build_capture_config(&args, &defaults(Some("/dev/video1"), Some("Y8"), Some(3)));

        match config.device {
            DeviceLocator::Path(path) => assert_eq!(path, PathBuf::from("/dev/video9")),
            DeviceLocator::Index(_) => panic!("expected path"),
        }
        assert_eq!(config.pixel_format, "GREY");
        assert_eq!(config.warmup_frames, 7);
    }

    #[test]
    fn config_defaults_apply_when_cli_missing() {
        let args = base_args();
        let config =
            build_capture_config(&args, &defaults(Some("/dev/video3"), Some("Y8"), Some(6)));

        match config.device {
            DeviceLocator::Path(path) => assert_eq!(path, PathBuf::from("/dev/video3")),
            other => panic!("unexpected device: {:?}", other),
        }
        assert_eq!(config.pixel_format, "Y8");
        assert_eq!(config.warmup_frames, 6);
    }

    #[test]
    fn built_in_defaults_cover_missing_config() {
        let args = base_args();
        let config = build_capture_config(&args, &defaults(None, None, None));

        match config.device {
            DeviceLocator::Index(idx) => assert_eq!(idx, 0),
            other => panic!("unexpected device: {:?}", other),
        }
        assert_eq!(config.pixel_format, DEFAULT_PIXEL_FORMAT);
        assert_eq!(config.warmup_frames, DEFAULT_WARMUP_FRAMES);
    }
}
