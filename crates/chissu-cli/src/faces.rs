pub use chissu_face_core::faces::*;

use crate::cli::{FaceCompareArgs, FaceEnrollArgs, FaceExtractArgs, FaceRemoveArgs};

impl From<&FaceExtractArgs> for FaceExtractionConfig {
    fn from(args: &FaceExtractArgs) -> Self {
        Self {
            image: args.image.clone(),
            landmark_model: args.landmark_model.clone(),
            encoder_model: args.encoder_model.clone(),
            output: args.output.clone(),
            jitters: args.jitters,
        }
    }
}

impl From<&FaceCompareArgs> for FaceComparisonConfig {
    fn from(args: &FaceCompareArgs) -> Self {
        Self {
            input: args.input.clone(),
            compare_targets: args.compare_targets.clone(),
        }
    }
}

impl From<&FaceEnrollArgs> for FaceEnrollmentConfig {
    fn from(args: &FaceEnrollArgs) -> Self {
        Self {
            user: args.user.clone(),
            descriptor: args.descriptor.clone(),
            store_dir: args.store_dir.clone(),
        }
    }
}

impl From<&FaceRemoveArgs> for FaceRemovalConfig {
    fn from(args: &FaceRemoveArgs) -> Self {
        Self {
            user: args.user.clone(),
            descriptor_ids: args.descriptor_id.clone(),
            remove_all: args.all,
            store_dir: args.store_dir.clone(),
        }
    }
}
