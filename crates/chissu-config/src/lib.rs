use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;

pub const PRIMARY_CONFIG_PATH: &str = "/etc/chissu-pam/config.toml";
pub const SECONDARY_CONFIG_PATH: &str = "/usr/local/etc/chissu-pam/config.toml";
pub const DEFAULT_SIMILARITY_THRESHOLD: f64 = 0.9;
pub const DEFAULT_TIMEOUT_SECS: u64 = 5;
pub const DEFAULT_INTERVAL_MILLIS: u64 = 500;
pub const DEFAULT_VIDEO_DEVICE: &str = "/dev/video0";
pub const DEFAULT_STORE_DIR: &str = "/var/lib/chissu-pam/models";
pub const DEFAULT_PIXEL_FORMAT: &str = "Y16";
pub const DEFAULT_WARMUP_FRAMES: u32 = 0;
pub const DEFAULT_JITTERS: u32 = 1;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigFile {
    pub similarity_threshold: Option<f64>,
    pub capture_timeout_secs: Option<u64>,
    pub frame_interval_millis: Option<u64>,
    pub embedding_store_dir: Option<PathBuf>,
    pub video_device: Option<String>,
    pub pixel_format: Option<String>,
    pub warmup_frames: Option<u32>,
    pub jitters: Option<u32>,
    pub landmark_model: Option<PathBuf>,
    pub encoder_model: Option<PathBuf>,
    pub require_secret_service: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub similarity_threshold: f64,
    pub capture_timeout: Duration,
    pub frame_interval: Duration,
    pub embedding_store_dir: PathBuf,
    pub video_device: String,
    pub pixel_format: String,
    pub warmup_frames: u32,
    pub jitters: u32,
    pub landmark_model: Option<PathBuf>,
    pub encoder_model: Option<PathBuf>,
    pub require_secret_service: bool,
}

impl ResolvedConfig {
    pub fn from_raw(raw: ConfigFile) -> Self {
        Self {
            similarity_threshold: raw
                .similarity_threshold
                .unwrap_or(DEFAULT_SIMILARITY_THRESHOLD),
            capture_timeout: Duration::from_secs(
                raw.capture_timeout_secs
                    .unwrap_or(DEFAULT_TIMEOUT_SECS)
                    .max(1),
            ),
            frame_interval: Duration::from_millis(
                raw.frame_interval_millis.unwrap_or(DEFAULT_INTERVAL_MILLIS),
            ),
            embedding_store_dir: raw
                .embedding_store_dir
                .unwrap_or_else(|| PathBuf::from(DEFAULT_STORE_DIR)),
            video_device: raw
                .video_device
                .unwrap_or_else(|| DEFAULT_VIDEO_DEVICE.to_string()),
            pixel_format: raw
                .pixel_format
                .unwrap_or_else(|| DEFAULT_PIXEL_FORMAT.to_string()),
            warmup_frames: raw.warmup_frames.unwrap_or(DEFAULT_WARMUP_FRAMES),
            jitters: raw.jitters.unwrap_or(DEFAULT_JITTERS),
            landmark_model: raw.landmark_model,
            encoder_model: raw.encoder_model,
            require_secret_service: raw.require_secret_service.unwrap_or(false),
        }
    }
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        Self::from_raw(ConfigFile::default())
    }
}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub contents: ConfigFile,
    pub source: PathBuf,
}

impl LoadedConfig {
    pub fn new(contents: ConfigFile, source: PathBuf) -> Self {
        Self { contents, source }
    }

    pub fn into_parts(self) -> (ConfigFile, PathBuf) {
        (self.contents, self.source)
    }

    pub fn into_contents(self) -> ConfigFile {
        self.contents
    }

    pub fn source(&self) -> &Path {
        &self.source
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedConfigWithSource {
    pub resolved: ResolvedConfig,
    pub source: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to parse {path}: {message}")]
    Parse { path: PathBuf, message: String },
}

pub fn load_config() -> Result<Option<LoadedConfig>, ConfigError> {
    let sources = [
        PathBuf::from(PRIMARY_CONFIG_PATH),
        PathBuf::from(SECONDARY_CONFIG_PATH),
    ];
    load_from_paths(&sources)
}

pub fn load_resolved_config() -> Result<ResolvedConfigWithSource, ConfigError> {
    let sources = [
        PathBuf::from(PRIMARY_CONFIG_PATH),
        PathBuf::from(SECONDARY_CONFIG_PATH),
    ];
    load_resolved_from_paths(&sources)
}

pub fn load_from_paths(paths: &[PathBuf]) -> Result<Option<LoadedConfig>, ConfigError> {
    for path in paths {
        match fs::read_to_string(path) {
            Ok(contents) => {
                let parsed =
                    toml::from_str::<ConfigFile>(&contents).map_err(|err| ConfigError::Parse {
                        path: path.clone(),
                        message: err.to_string(),
                    })?;
                return Ok(Some(LoadedConfig::new(parsed, path.clone())));
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(ConfigError::Read {
                    path: path.clone(),
                    source: err,
                })
            }
        }
    }

    Ok(None)
}

pub fn load_resolved_from_paths(
    paths: &[PathBuf],
) -> Result<ResolvedConfigWithSource, ConfigError> {
    match load_from_paths(paths)? {
        Some(entry) => {
            let path = entry.source.clone();
            Ok(ResolvedConfigWithSource {
                resolved: ResolvedConfig::from_raw(entry.contents),
                source: Some(path),
            })
        }
        None => Ok(ResolvedConfigWithSource {
            resolved: ResolvedConfig::default(),
            source: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn primary_path_wins() {
        let dir = tempdir().unwrap();
        let primary = dir.path().join("primary.toml");
        let secondary = dir.path().join("secondary.toml");
        fs::write(&secondary, "warmup_frames = 2").unwrap();
        fs::write(&primary, "warmup_frames = 5").unwrap();

        let loaded = load_from_paths(&[primary.clone(), secondary.clone()])
            .unwrap()
            .expect("config expected");
        assert_eq!(loaded.source, primary);
        assert_eq!(loaded.contents.warmup_frames, Some(5));
    }

    #[test]
    fn secondary_used_when_primary_missing() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing.toml");
        let secondary = dir.path().join("secondary.toml");
        fs::write(&secondary, "pixel_format = \"GREY\"").unwrap();

        let loaded = load_from_paths(&[missing.clone(), secondary.clone()])
            .unwrap()
            .expect("config expected");
        assert_eq!(loaded.source, secondary);
        assert_eq!(loaded.contents.pixel_format.as_deref(), Some("GREY"));
    }

    #[test]
    fn parse_errors_are_reported() {
        let dir = tempdir().unwrap();
        let broken = dir.path().join("broken.toml");
        fs::write(&broken, "embedding_store_dir = { invalid = true }").unwrap();

        let err = load_from_paths(&[broken.clone()]).unwrap_err();
        match err {
            ConfigError::Parse { path, .. } => assert_eq!(path, broken),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn io_errors_are_reported() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("dir.toml");
        fs::create_dir_all(&path).unwrap();

        let err = load_from_paths(&[path.clone()]).unwrap_err();
        match err {
            ConfigError::Read { path: err_path, .. } => assert_eq!(err_path, path),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn empty_paths_return_none() {
        let loaded = load_from_paths(&[]).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn resolved_defaults_apply_when_missing() {
        let resolved = load_resolved_from_paths(&[]).unwrap();
        assert!(resolved.source.is_none());
        assert_eq!(
            resolved.resolved.similarity_threshold,
            DEFAULT_SIMILARITY_THRESHOLD
        );
        assert_eq!(resolved.resolved.video_device, DEFAULT_VIDEO_DEVICE);
        assert_eq!(resolved.resolved.warmup_frames, DEFAULT_WARMUP_FRAMES);
    }

    #[test]
    fn resolved_config_reports_source() {
        let dir = tempdir().unwrap();
        let primary = dir.path().join("primary.toml");
        fs::write(&primary, "capture_timeout_secs = 10").unwrap();

        let resolved = load_resolved_from_paths(&[primary.clone()]).unwrap();
        assert_eq!(resolved.source, Some(primary));
        assert_eq!(resolved.resolved.capture_timeout, Duration::from_secs(10));
    }
}
