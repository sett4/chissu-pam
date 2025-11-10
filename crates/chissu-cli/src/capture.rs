pub use chissu_face_core::capture::*;

use crate::cli::{CaptureArgs, DEFAULT_PIXEL_FORMAT, DEFAULT_WARMUP_FRAMES};
use crate::config::CaptureDefaults;

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
