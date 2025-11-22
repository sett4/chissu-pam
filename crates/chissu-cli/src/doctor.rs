use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use chissu_config::{
    self, ConfigError, ResolvedConfig, ResolvedConfigWithSource, PRIMARY_CONFIG_PATH,
    SECONDARY_CONFIG_PATH,
};
use chissu_face_core::capture::DeviceLocator;
use chissu_face_core::secret_service::{
    default_service_name, ensure_secret_service_available, KeyringSecretServiceProbe,
    SecretServiceProbe,
};
use serde::Serialize;

use crate::errors::AppResult;

const CHECK_CONFIG: &str = "config";
const CHECK_VIDEO_DEVICE: &str = "video_device";
const CHECK_EMBEDDING_DIR: &str = "embedding_store_dir";
const CHECK_LANDMARK_MODEL: &str = "landmark_model";
const CHECK_ENCODER_MODEL: &str = "encoder_model";
const CHECK_SECRET_SERVICE: &str = "secret_service";
const CHECK_PAM_MODULE: &str = "pam_module";
const CHECK_PAM_STACK: &str = "pam_stack";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorOutcome {
    pub ok: bool,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Clone)]
pub struct DoctorPaths {
    pub config_paths: Vec<PathBuf>,
    pub pam_module_paths: Vec<PathBuf>,
    pub pamd_dir: PathBuf,
}

impl Default for DoctorPaths {
    fn default() -> Self {
        Self {
            config_paths: vec![
                PathBuf::from(PRIMARY_CONFIG_PATH),
                PathBuf::from(SECONDARY_CONFIG_PATH),
            ],
            pam_module_paths: vec![
                PathBuf::from("/usr/lib/x86_64-linux-gnu/security/libpam_chissu.so"),
                PathBuf::from("/lib/security/libpam_chissu.so"),
                PathBuf::from("/lib64/security/libpam_chissu.so"),
            ],
            pamd_dir: PathBuf::from("/etc/pam.d"),
        }
    }
}

pub trait DeviceOpener {
    fn open(&self, locator: &DeviceLocator) -> AppResult<()>;
}

#[derive(Clone, Copy)]
pub struct RealDeviceOpener;

impl DeviceOpener for RealDeviceOpener {
    fn open(&self, locator: &DeviceLocator) -> AppResult<()> {
        let _ = locator.open()?;
        Ok(())
    }
}

pub struct DoctorContext<P, D> {
    pub paths: DoctorPaths,
    pub secret_service_probe: P,
    pub device_opener: D,
    pub fallback_config: ResolvedConfig,
}

impl Default for DoctorContext<KeyringSecretServiceProbe, RealDeviceOpener> {
    fn default() -> Self {
        Self {
            paths: DoctorPaths::default(),
            secret_service_probe: KeyringSecretServiceProbe,
            device_opener: RealDeviceOpener,
            fallback_config: ResolvedConfig::default(),
        }
    }
}

pub fn run_doctor() -> AppResult<DoctorOutcome> {
    let ctx = DoctorContext::default();
    run_doctor_with(&ctx)
}

pub fn run_doctor_with<P, D>(ctx: &DoctorContext<P, D>) -> AppResult<DoctorOutcome>
where
    P: SecretServiceProbe,
    D: DeviceOpener,
{
    let (config_check, resolved) = check_config(&ctx.paths, &ctx.fallback_config);

    let mut checks = vec![config_check];

    checks.push(check_video_device(&resolved, &ctx.device_opener));
    checks.push(check_embedding_dir(&resolved));
    checks.push(check_model(
        CHECK_LANDMARK_MODEL,
        resolved.resolved.landmark_model.as_ref(),
    ));
    checks.push(check_model(
        CHECK_ENCODER_MODEL,
        resolved.resolved.encoder_model.as_ref(),
    ));
    checks.push(check_secret_service(&ctx.secret_service_probe));
    let (pam_stack_check, referenced_modules) = check_pam_stack(&ctx.paths.pamd_dir);
    checks.push(pam_stack_check);
    checks.push(check_pam_module(
        referenced_modules.as_slice(),
        &ctx.paths.pam_module_paths,
    ));

    let ok = checks.iter().all(|c| c.status == CheckStatus::Pass);

    Ok(DoctorOutcome { ok, checks })
}

fn check_config(
    paths: &DoctorPaths,
    fallback: &ResolvedConfig,
) -> (DoctorCheck, ResolvedConfigWithSource) {
    let primary_exists = paths
        .config_paths
        .first()
        .map(|p| p.exists())
        .unwrap_or(false);
    let secondary_exists = paths
        .config_paths
        .get(1)
        .map(|p| p.exists())
        .unwrap_or(false);

    let loaded = chissu_config::load_from_paths(&paths.config_paths);
    match loaded {
        Ok(Some(entry)) => {
            let resolved = ResolvedConfigWithSource {
                resolved: ResolvedConfig::from_raw(entry.contents.clone()),
                source: Some(entry.source.clone()),
            };
            if primary_exists && secondary_exists && entry.source == paths.config_paths[1] {
                return (
                    DoctorCheck {
                        name: CHECK_CONFIG.into(),
                        status: CheckStatus::Warn,
                        message: format!(
                            "Both config files exist; using secondary {}",
                            entry.source.display()
                        ),
                        path: Some(entry.source.display().to_string()),
                        device: None,
                    },
                    resolved,
                );
            }
            if primary_exists && secondary_exists {
                return (
                    DoctorCheck {
                        name: CHECK_CONFIG.into(),
                        status: CheckStatus::Warn,
                        message: format!(
                            "Both config files exist; using primary {}",
                            entry.source.display()
                        ),
                        path: Some(entry.source.display().to_string()),
                        device: None,
                    },
                    resolved,
                );
            }
            (
                DoctorCheck {
                    name: CHECK_CONFIG.into(),
                    status: CheckStatus::Pass,
                    message: format!("Loaded config from {}", entry.source.display()),
                    path: Some(entry.source.display().to_string()),
                    device: None,
                },
                resolved,
            )
        }
        Ok(None) => (
            DoctorCheck {
                name: CHECK_CONFIG.into(),
                status: CheckStatus::Fail,
                message: format!(
                    "Config file missing; tried {}",
                    display_paths(&paths.config_paths)
                ),
                path: None,
                device: None,
            },
            ResolvedConfigWithSource {
                resolved: fallback.clone(),
                source: None,
            },
        ),
        Err(ConfigError::Parse { path, message }) => (
            DoctorCheck {
                name: CHECK_CONFIG.into(),
                status: CheckStatus::Fail,
                message: format!("Failed to parse {}: {}", path.display(), message),
                path: Some(path.display().to_string()),
                device: None,
            },
            ResolvedConfigWithSource {
                resolved: fallback.clone(),
                source: None,
            },
        ),
        Err(ConfigError::Read { path, source }) => (
            DoctorCheck {
                name: CHECK_CONFIG.into(),
                status: CheckStatus::Fail,
                message: format!("Failed to read {}: {}", path.display(), source),
                path: Some(path.display().to_string()),
                device: None,
            },
            ResolvedConfigWithSource {
                resolved: fallback.clone(),
                source: None,
            },
        ),
    }
}

fn check_video_device<D: DeviceOpener>(cfg: &ResolvedConfigWithSource, opener: &D) -> DoctorCheck {
    let locator = DeviceLocator::from_option(Some(cfg.resolved.video_device.clone()));
    let display = display_device(&locator);

    match opener.open(&locator) {
        Ok(_) => DoctorCheck {
            name: CHECK_VIDEO_DEVICE.into(),
            status: CheckStatus::Pass,
            message: format!("Opened video device {display}"),
            path: None,
            device: Some(display),
        },
        Err(err) => DoctorCheck {
            name: CHECK_VIDEO_DEVICE.into(),
            status: CheckStatus::Fail,
            message: err.human_message(),
            path: None,
            device: Some(display),
        },
    }
}

fn check_embedding_dir(cfg: &ResolvedConfigWithSource) -> DoctorCheck {
    let path = &cfg.resolved.embedding_store_dir;
    match (path.exists(), path.is_dir()) {
        (false, _) => DoctorCheck {
            name: CHECK_EMBEDDING_DIR.into(),
            status: CheckStatus::Fail,
            message: format!("Embedding store {} missing", path.display()),
            path: Some(path.display().to_string()),
            device: None,
        },
        (true, false) => DoctorCheck {
            name: CHECK_EMBEDDING_DIR.into(),
            status: CheckStatus::Fail,
            message: format!("Embedding store {} is not a directory", path.display()),
            path: Some(path.display().to_string()),
            device: None,
        },
        (true, true) => {
            let readable = fs::read_dir(path).is_ok();
            let writeable = is_writeable_dir(path);
            if readable && writeable {
                DoctorCheck {
                    name: CHECK_EMBEDDING_DIR.into(),
                    status: CheckStatus::Pass,
                    message: format!("Embedding store {} is readable/writable", path.display()),
                    path: Some(path.display().to_string()),
                    device: None,
                }
            } else {
                DoctorCheck {
                    name: CHECK_EMBEDDING_DIR.into(),
                    status: CheckStatus::Fail,
                    message: format!(
                        "Embedding store {} lacks {} permissions",
                        path.display(),
                        if !readable && !writeable {
                            "read/write"
                        } else if !readable {
                            "read"
                        } else {
                            "write"
                        }
                    ),
                    path: Some(path.display().to_string()),
                    device: None,
                }
            }
        }
    }
}

fn check_model(name: &str, path: Option<&PathBuf>) -> DoctorCheck {
    match path {
        None => DoctorCheck {
            name: name.into(),
            status: CheckStatus::Fail,
            message: "Model path not configured; set config or env".into(),
            path: None,
            device: None,
        },
        Some(p) => match fs::File::open(p) {
            Ok(_) => DoctorCheck {
                name: name.into(),
                status: CheckStatus::Pass,
                message: format!("Found model at {}", p.display()),
                path: Some(p.display().to_string()),
                device: None,
            },
            Err(err) => DoctorCheck {
                name: name.into(),
                status: CheckStatus::Fail,
                message: format!("Cannot read model {}: {}", p.display(), err),
                path: Some(p.display().to_string()),
                device: None,
            },
        },
    }
}

fn check_secret_service<P: SecretServiceProbe>(probe: &P) -> DoctorCheck {
    let user = whoami::username();
    match ensure_secret_service_available(probe, &user) {
        Ok(_) => DoctorCheck {
            name: CHECK_SECRET_SERVICE.into(),
            status: CheckStatus::Pass,
            message: format!(
                "Secret Service available for user {} (service {})",
                user,
                default_service_name()
            ),
            path: None,
            device: None,
        },
        Err(err) => DoctorCheck {
            name: CHECK_SECRET_SERVICE.into(),
            status: CheckStatus::Fail,
            message: err.to_string(),
            path: None,
            device: None,
        },
    }
}

fn check_pam_module(referenced: &[PathBuf], fallback_paths: &[PathBuf]) -> DoctorCheck {
    let mut targets: Vec<PathBuf> = if referenced.is_empty() {
        fallback_paths.to_vec()
    } else {
        referenced.to_vec()
    };
    targets.dedup();

    let mut failures = Vec::new();
    let mut passes = Vec::new();

    for path in &targets {
        match fs::File::open(path) {
            Ok(file) => {
                if let Ok(metadata) = file.metadata() {
                    let mode = metadata.permissions().mode();
                    if mode & 0o022 != 0 {
                        failures.push(format!(
                            "{} is world/group-writable (mode {:o})",
                            path.display(),
                            mode & 0o777
                        ));
                        continue;
                    }
                }
                passes.push(path.display().to_string());
            }
            Err(err) => failures.push(format!("{}: {}", path.display(), err)),
        }
    }

    if passes.is_empty() {
        return DoctorCheck {
            name: CHECK_PAM_MODULE.into(),
            status: if referenced.is_empty() {
                CheckStatus::Warn
            } else {
                CheckStatus::Fail
            },
            message: if referenced.is_empty() {
                format!(
                    "No pam_chissu reference to validate; searched {}",
                    display_paths(&targets)
                )
            } else {
                format!("PAM module validation failed: {}", failures.join("; "))
            },
            path: None,
            device: None,
        };
    }

    if failures.is_empty() {
        DoctorCheck {
            name: CHECK_PAM_MODULE.into(),
            status: CheckStatus::Pass,
            message: format!("Validated PAM module(s): {}", passes.join(", ")),
            path: Some(passes.join(", ")),
            device: None,
        }
    } else {
        DoctorCheck {
            name: CHECK_PAM_MODULE.into(),
            status: CheckStatus::Fail,
            message: format!(
                "Some PAM modules invalid: {}; ok: {}",
                failures.join(", "),
                passes.join(", ")
            ),
            path: None,
            device: None,
        }
    }
}

fn check_pam_stack(pamd_dir: &Path) -> (DoctorCheck, Vec<PathBuf>) {
    match fs::read_dir(pamd_dir) {
        Ok(entries) => {
            let mut matched = Vec::new();
            let mut referenced = Vec::new();
            for entry in entries.flatten() {
                if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    continue;
                }
                if let Ok(contents) = fs::read_to_string(entry.path()) {
                    if file_references_pam_chissu(&contents, &mut referenced) {
                        matched.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
            }
            if matched.is_empty() {
                (
                    DoctorCheck {
                        name: CHECK_PAM_STACK.into(),
                        status: CheckStatus::Warn,
                        message: format!(
                            "No pam_chissu entry found under {}; add to your target service file",
                            pamd_dir.display()
                        ),
                        path: Some(pamd_dir.display().to_string()),
                        device: None,
                    },
                    referenced,
                )
            } else {
                (
                    DoctorCheck {
                        name: CHECK_PAM_STACK.into(),
                        status: CheckStatus::Pass,
                        message: format!("libpam_chissu.so referenced in: {}", matched.join(", ")),
                        path: Some(pamd_dir.display().to_string()),
                        device: None,
                    },
                    referenced,
                )
            }
        }
        Err(err) => (
            DoctorCheck {
                name: CHECK_PAM_STACK.into(),
                status: CheckStatus::Fail,
                message: format!("Failed to read {}: {}", pamd_dir.display(), err),
                path: Some(pamd_dir.display().to_string()),
                device: None,
            },
            Vec::new(),
        ),
    }
}

fn display_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn file_references_pam_chissu(contents: &str, referenced: &mut Vec<PathBuf>) -> bool {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let active = trimmed
            .split('#')
            .next()
            .map(|part| part.trim())
            .unwrap_or("");
        if active.contains("pam_chissu") {
            referenced.extend(module_paths_from_line(active));
            return true;
        }
    }
    false
}

fn module_paths_from_line(line: &str) -> Vec<PathBuf> {
    let mut tokens = line.split_whitespace();
    let _ = tokens.next(); // type
                           // controls may occupy multiple tokens (e.g., [success=3 default=ignore])
    let mut paths = Vec::new();
    for token in tokens {
        if token.starts_with('/') && token.contains("pam_chissu") {
            paths.push(PathBuf::from(token));
        }
    }
    paths
}

fn display_device(locator: &DeviceLocator) -> String {
    match locator {
        DeviceLocator::Index(i) => format!("/dev/video{i}"),
        DeviceLocator::Path(p) => p.display().to_string(),
    }
}

fn is_writeable_dir(path: &Path) -> bool {
    // Quick check: if read-only bit set, consider not writeable
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.permissions().readonly() {
            return false;
        }
    }

    // Best-effort POSIX access check without creating files
    let c_path = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
        Ok(c) => c,
        Err(_) => return false,
    };
    unsafe { libc::access(c_path.as_ptr(), libc::W_OK) == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::AppError;
    use chissu_face_core::secret_service::SecretServiceError;
    use std::fs::File;
    use std::io;
    use tempfile::tempdir;

    #[derive(Clone)]
    struct StubDeviceOpener {
        ok: bool,
    }

    impl DeviceOpener for StubDeviceOpener {
        fn open(&self, locator: &DeviceLocator) -> AppResult<()> {
            if self.ok {
                Ok(())
            } else {
                Err(AppError::DeviceOpen {
                    device: display_device(locator),
                    source: io::Error::from(io::ErrorKind::NotFound),
                })
            }
        }
    }

    #[derive(Clone)]
    struct StubProbe {
        result: Result<(), SecretServiceError>,
    }

    impl SecretServiceProbe for StubProbe {
        fn check(&self, _user: &str) -> Result<(), SecretServiceError> {
            self.result.clone()
        }
    }

    fn base_paths(tmp: &Path) -> DoctorPaths {
        DoctorPaths {
            config_paths: vec![tmp.join("config.toml")],
            pam_module_paths: vec![tmp.join("libpam_chissu.so")],
            pamd_dir: tmp.join("pam.d"),
        }
    }

    fn resolved_for(tmp: &Path) -> ResolvedConfig {
        let raw = chissu_config::ConfigFile {
            embedding_store_dir: Some(tmp.join("store")),
            video_device: Some(tmp.join("video0").display().to_string()),
            pixel_format: None,
            warmup_frames: None,
            jitters: None,
            landmark_model: Some(tmp.join("landmark.dat")),
            encoder_model: Some(tmp.join("encoder.dat")),
            similarity_threshold: None,
            capture_timeout_secs: None,
            frame_interval_millis: None,
            require_secret_service: None,
        };
        ResolvedConfig::from_raw(raw)
    }

    fn write_fixtures(tmp: &Path) {
        fs::create_dir_all(tmp.join("store")).unwrap();
        fs::create_dir_all(tmp.join("pam.d")).unwrap();
        File::create(tmp.join("landmark.dat")).unwrap();
        File::create(tmp.join("encoder.dat")).unwrap();
        File::create(tmp.join("pam.d/login")).unwrap();
        fs::write(tmp.join("pam.d/login"), "auth required pam_chissu.so").unwrap();
        File::create(tmp.join("video0")).unwrap();
        let module = tmp.join("libpam_chissu.so");
        File::create(&module).unwrap();
        fs::set_permissions(&module, fs::Permissions::from_mode(0o644)).unwrap();
    }

    fn doctor_with(
        paths: DoctorPaths,
        probe: StubProbe,
        device_ok: bool,
        fallback: ResolvedConfig,
    ) -> DoctorOutcome {
        let ctx = DoctorContext {
            paths,
            secret_service_probe: probe,
            device_opener: StubDeviceOpener { ok: device_ok },
            fallback_config: fallback,
        };
        run_doctor_with(&ctx).unwrap()
    }

    fn status<'a>(checks: &'a [DoctorCheck], name: &str) -> &'a DoctorCheck {
        checks
            .iter()
            .find(|c| c.name == name)
            .expect("check present")
    }

    #[test]
    fn doctor_success_when_all_checks_pass() {
        let tmp = tempdir().unwrap();
        write_fixtures(tmp.path());

        fs::write(
            tmp.path().join("config.toml"),
            format!(
                "embedding_store_dir = \"{}\"\nvideo_device = \"{}\"\nlandmark_model = \"{}\"\nencoder_model = \"{}\"\n",
                tmp.path().join("store").display(),
                tmp.path().join("video0").display(),
                tmp.path().join("landmark.dat").display(),
                tmp.path().join("encoder.dat").display()
            ),
        )
        .unwrap();

        let outcome = doctor_with(
            base_paths(tmp.path()),
            StubProbe { result: Ok(()) },
            true,
            resolved_for(tmp.path()),
        );

        assert!(outcome.ok, "statuses: {:?}", outcome.checks);
        assert_eq!(
            status(&outcome.checks, CHECK_CONFIG).status,
            CheckStatus::Pass
        );
    }

    #[test]
    fn doctor_reports_missing_config() {
        let tmp = tempdir().unwrap();
        write_fixtures(tmp.path());
        let outcome = doctor_with(
            base_paths(tmp.path()),
            StubProbe { result: Ok(()) },
            true,
            resolved_for(tmp.path()),
        );

        assert_eq!(
            status(&outcome.checks, CHECK_CONFIG).status,
            CheckStatus::Fail
        );
        assert!(!outcome.ok);
    }

    #[test]
    fn doctor_reports_parse_error() {
        let tmp = tempdir().unwrap();
        write_fixtures(tmp.path());
        fs::write(
            tmp.path().join("config.toml"),
            "embedding_store_dir = { invalid = true }",
        )
        .unwrap();

        let outcome = doctor_with(
            base_paths(tmp.path()),
            StubProbe { result: Ok(()) },
            true,
            resolved_for(tmp.path()),
        );

        assert_eq!(
            status(&outcome.checks, CHECK_CONFIG).status,
            CheckStatus::Fail
        );
        assert!(!outcome.ok);
    }

    #[test]
    fn doctor_reports_missing_device() {
        let tmp = tempdir().unwrap();
        write_fixtures(tmp.path());
        fs::write(
            tmp.path().join("config.toml"),
            format!(
                "embedding_store_dir = \"{}\"\nvideo_device = \"{}\"\nlandmark_model = \"{}\"\nencoder_model = \"{}\"\n",
                tmp.path().join("store").display(),
                tmp.path().join("video0").display(),
                tmp.path().join("landmark.dat").display(),
                tmp.path().join("encoder.dat").display()
            ),
        )
        .unwrap();

        let outcome = doctor_with(
            base_paths(tmp.path()),
            StubProbe { result: Ok(()) },
            false,
            resolved_for(tmp.path()),
        );

        assert_eq!(
            status(&outcome.checks, CHECK_VIDEO_DEVICE).status,
            CheckStatus::Fail
        );
        assert!(!outcome.ok);
    }

    #[test]
    fn doctor_reports_missing_pam_module_and_stack() {
        let tmp = tempdir().unwrap();
        write_fixtures(tmp.path());
        fs::write(
            tmp.path().join("config.toml"),
            format!(
                "embedding_store_dir = \"{}\"\nvideo_device = \"{}\"\nlandmark_model = \"{}\"\nencoder_model = \"{}\"\n",
                tmp.path().join("store").display(),
                tmp.path().join("video0").display(),
                tmp.path().join("landmark.dat").display(),
                tmp.path().join("encoder.dat").display()
            ),
        )
        .unwrap();
        // remove pam module file and pam entry
        fs::remove_file(tmp.path().join("libpam_chissu.so")).unwrap();
        fs::write(tmp.path().join("pam.d/login"), "# no chissu here").unwrap();

        let outcome = doctor_with(
            base_paths(tmp.path()),
            StubProbe { result: Ok(()) },
            true,
            resolved_for(tmp.path()),
        );

        assert_eq!(
            status(&outcome.checks, CHECK_PAM_MODULE).status,
            CheckStatus::Warn
        );
        assert_eq!(
            status(&outcome.checks, CHECK_PAM_STACK).status,
            CheckStatus::Warn
        );
        assert!(!outcome.ok);
    }

    #[test]
    fn doctor_ignores_commented_pam_entries() {
        let tmp = tempdir().unwrap();
        write_fixtures(tmp.path());
        fs::write(
            tmp.path().join("pam.d/login"),
            "# auth sufficient pam_chissu.so\n",
        )
        .unwrap();
        fs::write(
            tmp.path().join("config.toml"),
            format!(
                "embedding_store_dir = \"{}\"\nvideo_device = \"{}\"\nlandmark_model = \"{}\"\nencoder_model = \"{}\"\n",
                tmp.path().join("store").display(),
                tmp.path().join("video0").display(),
                tmp.path().join("landmark.dat").display(),
                tmp.path().join("encoder.dat").display()
            ),
        )
        .unwrap();

        let outcome = doctor_with(
            base_paths(tmp.path()),
            StubProbe { result: Ok(()) },
            true,
            resolved_for(tmp.path()),
        );

        assert_eq!(
            status(&outcome.checks, CHECK_PAM_STACK).status,
            CheckStatus::Warn
        );
    }

    #[test]
    fn doctor_parses_complex_control_lines() {
        let tmp = tempdir().unwrap();
        write_fixtures(tmp.path());
        let module = tmp.path().join("libpam_chissu.so");
        fs::write(
            tmp.path().join("pam.d/login"),
            format!("auth [success=3 default=ignore] {}\n", module.display()),
        )
        .unwrap();
        fs::write(
            tmp.path().join("config.toml"),
            format!(
                "embedding_store_dir = \"{}\"\nvideo_device = \"{}\"\nlandmark_model = \"{}\"\nencoder_model = \"{}\"\n",
                tmp.path().join("store").display(),
                tmp.path().join("video0").display(),
                tmp.path().join("landmark.dat").display(),
                tmp.path().join("encoder.dat").display()
            ),
        )
        .unwrap();
        File::create(&module).unwrap();
        fs::set_permissions(&module, fs::Permissions::from_mode(0o644)).unwrap();

        let outcome = doctor_with(
            base_paths(tmp.path()),
            StubProbe { result: Ok(()) },
            true,
            resolved_for(tmp.path()),
        );

        assert_eq!(
            status(&outcome.checks, CHECK_PAM_STACK).status,
            CheckStatus::Pass
        );
        assert_eq!(
            status(&outcome.checks, CHECK_PAM_MODULE).status,
            CheckStatus::Pass
        );
    }
}
