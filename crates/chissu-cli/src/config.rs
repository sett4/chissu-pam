use std::path::PathBuf;

use chissu_config::{self, ConfigError, ConfigFile, PRIMARY_CONFIG_PATH, SECONDARY_CONFIG_PATH};

use crate::errors::{AppError, AppResult};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CaptureDefaults {
    pub device: Option<String>,
    pub pixel_format: Option<String>,
    pub warmup_frames: Option<u32>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FaceModelDefaults {
    pub landmark_model: Option<PathBuf>,
    pub encoder_model: Option<PathBuf>,
}

pub fn resolve_store_dir(cli_value: Option<PathBuf>) -> AppResult<Option<PathBuf>> {
    let sources = [
        PathBuf::from(PRIMARY_CONFIG_PATH),
        PathBuf::from(SECONDARY_CONFIG_PATH),
    ];
    resolve_store_dir_with_sources(cli_value, &sources)
}

fn resolve_store_dir_with_sources(
    cli_value: Option<PathBuf>,
    sources: &[PathBuf],
) -> AppResult<Option<PathBuf>> {
    if cli_value.is_some() {
        return Ok(cli_value);
    }
    Ok(load_config_from_paths(sources)?.and_then(|file| file.embedding_store_dir))
}

pub fn load_capture_defaults() -> AppResult<CaptureDefaults> {
    let sources = [
        PathBuf::from(PRIMARY_CONFIG_PATH),
        PathBuf::from(SECONDARY_CONFIG_PATH),
    ];
    load_capture_defaults_with_sources(&sources)
}

fn load_capture_defaults_with_sources(paths: &[PathBuf]) -> AppResult<CaptureDefaults> {
    Ok(
        load_config_from_paths(paths)?.map_or_else(CaptureDefaults::default, |file| {
            CaptureDefaults {
                device: file.video_device,
                pixel_format: file.pixel_format,
                warmup_frames: file.warmup_frames,
            }
        }),
    )
}

pub fn load_face_model_defaults() -> AppResult<FaceModelDefaults> {
    let sources = [
        PathBuf::from(PRIMARY_CONFIG_PATH),
        PathBuf::from(SECONDARY_CONFIG_PATH),
    ];
    load_face_model_defaults_with_sources(&sources)
}

fn load_face_model_defaults_with_sources(paths: &[PathBuf]) -> AppResult<FaceModelDefaults> {
    Ok(
        load_config_from_paths(paths)?.map_or_else(FaceModelDefaults::default, |file| {
            FaceModelDefaults {
                landmark_model: file.landmark_model,
                encoder_model: file.encoder_model,
            }
        }),
    )
}

fn load_config_from_paths(paths: &[PathBuf]) -> AppResult<Option<ConfigFile>> {
    chissu_config::load_from_paths(paths)
        .map(|loaded| loaded.map(|entry| entry.into_contents()))
        .map_err(map_config_error)
}

fn map_config_error(err: ConfigError) -> AppError {
    match err {
        ConfigError::Read { path, source } => AppError::ConfigRead { path, source },
        ConfigError::Parse { path, message } => AppError::ConfigParse { path, message },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn cli_value_wins_over_config() {
        let val = PathBuf::from("/tmp/custom");
        let resolved = resolve_store_dir_with_sources(Some(val.clone()), &[]).unwrap();
        assert_eq!(resolved.unwrap(), val);
    }

    #[test]
    fn primary_config_is_used_when_present() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, "embedding_store_dir = \"/srv/store\"").unwrap();

        let resolved = resolve_store_dir_with_sources(None, &[config_path.clone()]).unwrap();
        assert_eq!(resolved.unwrap(), PathBuf::from("/srv/store"));
    }

    #[test]
    fn secondary_used_when_primary_missing() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing.toml");
        let secondary = dir.path().join("secondary.toml");
        fs::write(&secondary, "embedding_store_dir = \"/var/tmp/store\"").unwrap();

        let resolved =
            resolve_store_dir_with_sources(None, &[missing.clone(), secondary.clone()]).unwrap();
        assert_eq!(resolved.unwrap(), PathBuf::from("/var/tmp/store"));
    }

    #[test]
    fn parse_error_is_reported() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("broken.toml");
        fs::write(&config_path, "embedding_store_dir = { not = 'toml' }").unwrap();

        let err = resolve_store_dir_with_sources(None, &[config_path.clone()]).unwrap_err();
        match err {
            AppError::ConfigParse { path, .. } => assert_eq!(path, config_path),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn io_error_is_reported() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::create_dir_all(&config_path).unwrap();

        let err = resolve_store_dir_with_sources(None, &[config_path.clone()]).unwrap_err();
        match err {
            AppError::ConfigRead { path, .. } => assert_eq!(path, config_path),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn empty_sources_result_in_none() {
        let resolved = resolve_store_dir_with_sources(None, &[]).unwrap();
        assert!(resolved.is_none());
    }

    #[test]
    fn capture_defaults_come_from_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            "video_device = \"/dev/video5\"\npixel_format = \"GREY\"\nwarmup_frames = 9\n",
        )
        .unwrap();

        let defaults = load_capture_defaults_with_sources(&[config_path.clone()]).unwrap();
        assert_eq!(
            defaults,
            CaptureDefaults {
                device: Some("/dev/video5".into()),
                pixel_format: Some("GREY".into()),
                warmup_frames: Some(9),
            }
        );
    }

    #[test]
    fn capture_defaults_parse_errors_surface() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("broken.toml");
        fs::write(&config_path, "pixel_format = { invalid = true }").unwrap();

        let err = load_capture_defaults_with_sources(&[config_path.clone()]).unwrap_err();
        match err {
            AppError::ConfigParse { path, .. } => assert_eq!(path, config_path),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn capture_defaults_read_errors_surface() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::create_dir_all(&config_path).unwrap();

        let err = load_capture_defaults_with_sources(&[config_path.clone()]).unwrap_err();
        match err {
            AppError::ConfigRead { path, .. } => assert_eq!(path, config_path),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn face_model_defaults_come_from_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            "landmark_model = \"/opt/landmark.dat\"\nencoder_model = \"/opt/encoder.dat\"\n",
        )
        .unwrap();

        let defaults = load_face_model_defaults_with_sources(&[config_path.clone()]).unwrap();
        assert_eq!(
            defaults,
            FaceModelDefaults {
                landmark_model: Some("/opt/landmark.dat".into()),
                encoder_model: Some("/opt/encoder.dat".into()),
            }
        );
    }

    #[test]
    fn face_model_defaults_missing_config_returns_empty() {
        let defaults = load_face_model_defaults_with_sources(&[]).unwrap();
        assert_eq!(defaults, FaceModelDefaults::default());
    }
}
