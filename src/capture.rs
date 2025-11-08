pub use chissu_face_core::capture::*;

use crate::cli::CaptureArgs;

impl From<&CaptureArgs> for CaptureConfig {
    fn from(args: &CaptureArgs) -> Self {
        CaptureConfig {
            device: DeviceLocator::from_option(args.device.clone()),
            pixel_format: args.pixel_format.clone(),
            width: args.width,
            height: args.height,
            exposure: args.exposure,
            gain: args.gain,
            auto_exposure: args.auto_exposure,
            auto_gain: args.auto_gain,
            warmup_frames: args.warmup_frames,
            output: args.output.clone(),
        }
    }
}
