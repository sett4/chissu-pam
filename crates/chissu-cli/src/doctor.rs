use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

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
const CHECK_POLKIT_HELPER_UNIT: &str = "polkit_helper_unit";
const CHECK_POLKIT_SESSION_BUS_ACCESS: &str = "polkit_session_bus_access";
const CHECK_POLKIT_VIDEO_DEVICE_ACCESS: &str = "polkit_video_device_access";
const POLKIT_HELPER_UNIT: &str = "polkit-agent-helper@.service";
const POLKIT_GUIDE: &str = "docs/users-guide/polkit-agent-helper-troubleshooting.md";

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

#[derive(Debug, Clone, Copy, Default)]
pub struct DoctorOptions {
    pub include_polkit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorProfile {
    Default,
    Polkit,
}

impl DoctorOptions {
    fn includes(self, profile: DoctorProfile) -> bool {
        match profile {
            DoctorProfile::Default => true,
            DoctorProfile::Polkit => self.include_polkit,
        }
    }
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

pub struct DoctorContext<P, D, I = RealPolkitInspector> {
    pub paths: DoctorPaths,
    pub secret_service_probe: P,
    pub device_opener: D,
    pub polkit_inspector: I,
    pub fallback_config: ResolvedConfig,
}

impl Default for DoctorContext<KeyringSecretServiceProbe, RealDeviceOpener> {
    fn default() -> Self {
        Self {
            paths: DoctorPaths::default(),
            secret_service_probe: KeyringSecretServiceProbe,
            device_opener: RealDeviceOpener,
            polkit_inspector: RealPolkitInspector,
            fallback_config: ResolvedConfig::default(),
        }
    }
}

struct DoctorServices<'a, P, D, I> {
    paths: &'a DoctorPaths,
    secret_service_probe: &'a P,
    device_opener: &'a D,
    polkit_inspector: &'a I,
    fallback_config: &'a ResolvedConfig,
}

impl<'a, P, D, I> From<&'a DoctorContext<P, D, I>> for DoctorServices<'a, P, D, I> {
    fn from(ctx: &'a DoctorContext<P, D, I>) -> Self {
        Self {
            paths: &ctx.paths,
            secret_service_probe: &ctx.secret_service_probe,
            device_opener: &ctx.device_opener,
            polkit_inspector: &ctx.polkit_inspector,
            fallback_config: &ctx.fallback_config,
        }
    }
}

#[derive(Default)]
struct DoctorState {
    resolved: Option<ResolvedConfigWithSource>,
    referenced_modules: Vec<PathBuf>,
    polkit_unit: Option<Result<PolkitUnitSettings, String>>,
    checks: Vec<DoctorCheck>,
}

impl DoctorState {
    fn push(&mut self, check: DoctorCheck) {
        self.checks.push(check);
    }

    fn resolved(&self) -> &ResolvedConfigWithSource {
        self.resolved
            .as_ref()
            .expect("config check must run before checks that need resolved config")
    }
}

struct DoctorCheckSpec<P, D, I> {
    name: &'static str,
    profiles: &'static [DoctorProfile],
    run: fn(&mut DoctorState, &DoctorServices<'_, P, D, I>),
}

impl<P, D, I> DoctorCheckSpec<P, D, I> {
    fn enabled(&self, options: DoctorOptions) -> bool {
        self.profiles
            .iter()
            .any(|profile| options.includes(*profile))
    }
}

pub fn run_doctor() -> AppResult<DoctorOutcome> {
    run_doctor_with_options(DoctorOptions::default())
}

pub fn run_doctor_with_options(options: DoctorOptions) -> AppResult<DoctorOutcome> {
    let ctx = DoctorContext::default();
    run_doctor_with_options_and_context(&ctx, options)
}

pub fn run_doctor_with<P, D, I>(ctx: &DoctorContext<P, D, I>) -> AppResult<DoctorOutcome>
where
    P: SecretServiceProbe,
    D: DeviceOpener,
    I: PolkitInspector,
{
    run_doctor_with_options_and_context(ctx, DoctorOptions::default())
}

pub fn run_doctor_with_options_and_context<P, D, I>(
    ctx: &DoctorContext<P, D, I>,
    options: DoctorOptions,
) -> AppResult<DoctorOutcome>
where
    P: SecretServiceProbe,
    D: DeviceOpener,
    I: PolkitInspector,
{
    let services = DoctorServices::from(ctx);
    let mut state = DoctorState::default();
    for spec in check_registry() {
        if spec.enabled(options) {
            debug_assert!(!spec.name.is_empty());
            (spec.run)(&mut state, &services);
        }
    }

    let ok = state.checks.iter().all(|c| c.status == CheckStatus::Pass);

    Ok(DoctorOutcome {
        ok,
        checks: state.checks,
    })
}

#[derive(Clone, Copy)]
pub struct RealPolkitInspector;

pub trait PolkitInspector {
    fn inspect(&self) -> Result<PolkitUnitSettings, String>;
}

impl PolkitInspector for RealPolkitInspector {
    fn inspect(&self) -> Result<PolkitUnitSettings, String> {
        inspect_polkit_unit()
    }
}

const DEFAULT_PROFILE: &[DoctorProfile] = &[DoctorProfile::Default];
const POLKIT_PROFILE: &[DoctorProfile] = &[DoctorProfile::Polkit];

fn check_registry<P, D, I>() -> [DoctorCheckSpec<P, D, I>; 11]
where
    P: SecretServiceProbe,
    D: DeviceOpener,
    I: PolkitInspector,
{
    [
        DoctorCheckSpec {
            name: CHECK_CONFIG,
            profiles: DEFAULT_PROFILE,
            run: run_config_check,
        },
        DoctorCheckSpec {
            name: CHECK_VIDEO_DEVICE,
            profiles: DEFAULT_PROFILE,
            run: run_video_device_check,
        },
        DoctorCheckSpec {
            name: CHECK_EMBEDDING_DIR,
            profiles: DEFAULT_PROFILE,
            run: run_embedding_dir_check,
        },
        DoctorCheckSpec {
            name: CHECK_LANDMARK_MODEL,
            profiles: DEFAULT_PROFILE,
            run: run_landmark_model_check,
        },
        DoctorCheckSpec {
            name: CHECK_ENCODER_MODEL,
            profiles: DEFAULT_PROFILE,
            run: run_encoder_model_check,
        },
        DoctorCheckSpec {
            name: CHECK_SECRET_SERVICE,
            profiles: DEFAULT_PROFILE,
            run: run_secret_service_check,
        },
        DoctorCheckSpec {
            name: CHECK_PAM_STACK,
            profiles: DEFAULT_PROFILE,
            run: run_pam_stack_check,
        },
        DoctorCheckSpec {
            name: CHECK_PAM_MODULE,
            profiles: DEFAULT_PROFILE,
            run: run_pam_module_check,
        },
        DoctorCheckSpec {
            name: CHECK_POLKIT_HELPER_UNIT,
            profiles: POLKIT_PROFILE,
            run: run_polkit_helper_unit_check,
        },
        DoctorCheckSpec {
            name: CHECK_POLKIT_SESSION_BUS_ACCESS,
            profiles: POLKIT_PROFILE,
            run: run_polkit_session_bus_access_check,
        },
        DoctorCheckSpec {
            name: CHECK_POLKIT_VIDEO_DEVICE_ACCESS,
            profiles: POLKIT_PROFILE,
            run: run_polkit_video_device_access_check,
        },
    ]
}

fn run_config_check<P, D, I>(state: &mut DoctorState, services: &DoctorServices<'_, P, D, I>) {
    let (check, resolved) = check_config(services.paths, services.fallback_config);
    state.resolved = Some(resolved);
    state.push(check);
}

fn run_video_device_check<P, D, I>(state: &mut DoctorState, services: &DoctorServices<'_, P, D, I>)
where
    D: DeviceOpener,
{
    state.push(check_video_device(state.resolved(), services.device_opener));
}

fn run_embedding_dir_check<P, D, I>(
    state: &mut DoctorState,
    _services: &DoctorServices<'_, P, D, I>,
) {
    state.push(check_embedding_dir(state.resolved()));
}

fn run_landmark_model_check<P, D, I>(
    state: &mut DoctorState,
    _services: &DoctorServices<'_, P, D, I>,
) {
    state.push(check_model(
        CHECK_LANDMARK_MODEL,
        state.resolved().resolved.landmark_model.as_ref(),
    ));
}

fn run_encoder_model_check<P, D, I>(
    state: &mut DoctorState,
    _services: &DoctorServices<'_, P, D, I>,
) {
    state.push(check_model(
        CHECK_ENCODER_MODEL,
        state.resolved().resolved.encoder_model.as_ref(),
    ));
}

fn run_secret_service_check<P, D, I>(
    state: &mut DoctorState,
    services: &DoctorServices<'_, P, D, I>,
) where
    P: SecretServiceProbe,
{
    state.push(check_secret_service(services.secret_service_probe));
}

fn run_pam_stack_check<P, D, I>(state: &mut DoctorState, services: &DoctorServices<'_, P, D, I>) {
    let (check, referenced_modules) = check_pam_stack(&services.paths.pamd_dir);
    state.referenced_modules = referenced_modules;
    state.push(check);
}

fn run_pam_module_check<P, D, I>(state: &mut DoctorState, services: &DoctorServices<'_, P, D, I>) {
    state.push(check_pam_module(
        state.referenced_modules.as_slice(),
        &services.paths.pam_module_paths,
    ));
}

fn run_polkit_helper_unit_check<P, D, I>(
    state: &mut DoctorState,
    services: &DoctorServices<'_, P, D, I>,
) where
    I: PolkitInspector,
{
    let check = match polkit_unit_result(state, services) {
        Ok(settings) => check_polkit_helper_unit(&settings),
        Err(message) => DoctorCheck {
            name: CHECK_POLKIT_HELPER_UNIT.into(),
            status: CheckStatus::Warn,
            message: format!(
                "Could not inspect {POLKIT_HELPER_UNIT}: {message}; see {POLKIT_GUIDE}"
            ),
            path: None,
            device: None,
        },
    };
    state.push(check);
}

fn run_polkit_session_bus_access_check<P, D, I>(
    state: &mut DoctorState,
    services: &DoctorServices<'_, P, D, I>,
) where
    I: PolkitInspector,
{
    let check = match polkit_unit_result(state, services) {
        Ok(settings) => check_polkit_session_bus_access(&settings),
        Err(_) => DoctorCheck {
            name: CHECK_POLKIT_SESSION_BUS_ACCESS.into(),
            status: CheckStatus::Warn,
            message: format!(
                "Skipped session bus sandbox check because {POLKIT_HELPER_UNIT} could not be inspected"
            ),
            path: Some("/run/user".into()),
            device: None,
        },
    };
    state.push(check);
}

fn run_polkit_video_device_access_check<P, D, I>(
    state: &mut DoctorState,
    services: &DoctorServices<'_, P, D, I>,
) where
    I: PolkitInspector,
{
    let device = display_device(&DeviceLocator::from_option(Some(
        state.resolved().resolved.video_device.clone(),
    )));
    let check = match polkit_unit_result(state, services) {
        Ok(settings) => check_polkit_video_device_access(&settings, &device),
        Err(_) => DoctorCheck {
            name: CHECK_POLKIT_VIDEO_DEVICE_ACCESS.into(),
            status: CheckStatus::Warn,
            message: format!(
                "Skipped video device sandbox check because {POLKIT_HELPER_UNIT} could not be inspected"
            ),
            path: None,
            device: Some(device),
        },
    };
    state.push(check);
}

fn polkit_unit_result<P, D, I>(
    state: &mut DoctorState,
    services: &DoctorServices<'_, P, D, I>,
) -> Result<PolkitUnitSettings, String>
where
    I: PolkitInspector,
{
    if state.polkit_unit.is_none() {
        state.polkit_unit = Some(services.polkit_inspector.inspect());
    }
    state
        .polkit_unit
        .as_ref()
        .expect("polkit unit inspection result initialized")
        .clone()
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
            let traversable = is_traversable_dir(path);
            let writeable = is_writeable_dir(path);
            if traversable && writeable {
                DoctorCheck {
                    name: CHECK_EMBEDDING_DIR.into(),
                    status: CheckStatus::Pass,
                    message: format!("Embedding store {} is traversable/writable", path.display()),
                    path: Some(path.display().to_string()),
                    device: None,
                }
            } else {
                DoctorCheck {
                    name: CHECK_EMBEDDING_DIR.into(),
                    status: CheckStatus::Fail,
                    message: format!(
                        "Embedding store {} lacks {} permissions (for shared enrollment use root:root mode 01733)",
                        path.display(),
                        if !traversable && !writeable {
                            "traverse/write"
                        } else if !traversable {
                            "traverse"
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

    let mut missing = Vec::new();
    let mut insecure = Vec::new();
    let mut passes = Vec::new();

    for path in &targets {
        match fs::File::open(path) {
            Ok(file) => {
                if let Ok(metadata) = file.metadata() {
                    let mode = metadata.permissions().mode();
                    if mode & 0o022 != 0 {
                        insecure.push(format!(
                            "{} is world/group-writable (mode {:o})",
                            path.display(),
                            mode & 0o777
                        ));
                        continue;
                    }
                }
                passes.push(path.display().to_string());
            }
            Err(err) => missing.push(format!("{}: {}", path.display(), err)),
        }
    }

    if !insecure.is_empty() {
        return DoctorCheck {
            name: CHECK_PAM_MODULE.into(),
            status: CheckStatus::Fail,
            message: format!("PAM module validation failed: {}", insecure.join("; ")),
            path: None,
            device: None,
        };
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
                format!("PAM module validation failed: {}", missing.join("; "))
            },
            path: None,
            device: None,
        };
    }

    if missing.is_empty() {
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
            status: CheckStatus::Pass,
            message: format!(
                "Validated PAM module(s): {}; ignored unavailable candidate(s): {}",
                passes.join(", "),
                missing.join("; ")
            ),
            path: Some(passes.join(", ")),
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PolkitUnitSettings {
    load_state: Option<String>,
    protect_home: Option<String>,
    bind_read_only_paths: Vec<String>,
    bind_paths: Vec<String>,
    private_devices: Option<String>,
    device_policy: Option<String>,
    device_allow: Vec<String>,
    source: String,
}

fn check_polkit_helper_unit(settings: &PolkitUnitSettings) -> DoctorCheck {
    if polkit_unit_unavailable(settings) {
        return DoctorCheck {
            name: CHECK_POLKIT_HELPER_UNIT.into(),
            status: CheckStatus::Warn,
            message: format!(
                "{POLKIT_HELPER_UNIT} load state is {}; polkit prompts may use a different helper on this system",
                settings.load_state.as_deref().unwrap_or("unknown")
            ),
            path: None,
            device: None,
        };
    }

    DoctorCheck {
        name: CHECK_POLKIT_HELPER_UNIT.into(),
        status: CheckStatus::Pass,
        message: format!("Inspected {POLKIT_HELPER_UNIT} via {}", settings.source),
        path: None,
        device: None,
    }
}

fn check_polkit_session_bus_access(settings: &PolkitUnitSettings) -> DoctorCheck {
    if polkit_unit_unavailable(settings) {
        return DoctorCheck {
            name: CHECK_POLKIT_SESSION_BUS_ACCESS.into(),
            status: CheckStatus::Warn,
            message: format!(
                "Skipped session bus sandbox check because {POLKIT_HELPER_UNIT} is not loaded"
            ),
            path: Some("/run/user".into()),
            device: None,
        };
    }

    let protect_home = settings.protect_home.as_deref().unwrap_or("unset");
    let hides_runtime = matches!(protect_home, "yes" | "read-only" | "tmpfs");
    let has_runtime_bind = contains_bound_path(&settings.bind_read_only_paths, "/run/user")
        || contains_bound_path(&settings.bind_paths, "/run/user");

    if hides_runtime && !has_runtime_bind {
        return DoctorCheck {
            name: CHECK_POLKIT_SESSION_BUS_ACCESS.into(),
            status: CheckStatus::Fail,
            message: format!(
                "{POLKIT_HELPER_UNIT} sets ProtectHome={protect_home} without BindReadOnlyPaths=/run/user; Secret Service bus access may fail. See {POLKIT_GUIDE}"
            ),
            path: Some("/run/user".into()),
            device: None,
        };
    }

    DoctorCheck {
        name: CHECK_POLKIT_SESSION_BUS_ACCESS.into(),
        status: CheckStatus::Pass,
        message: format!(
            "{POLKIT_HELPER_UNIT} exposes /run/user for recovered Secret Service session bus access"
        ),
        path: Some("/run/user".into()),
        device: None,
    }
}

fn check_polkit_video_device_access(settings: &PolkitUnitSettings, device: &str) -> DoctorCheck {
    if polkit_unit_unavailable(settings) {
        return DoctorCheck {
            name: CHECK_POLKIT_VIDEO_DEVICE_ACCESS.into(),
            status: CheckStatus::Warn,
            message: format!(
                "Skipped video device sandbox check because {POLKIT_HELPER_UNIT} is not loaded"
            ),
            path: None,
            device: Some(device.into()),
        };
    }

    let mut failures = Vec::new();
    if matches!(settings.private_devices.as_deref(), Some("yes" | "true"))
        && !contains_bound_path(&settings.bind_paths, device)
    {
        failures.push(format!("PrivateDevices=yes without BindPaths={device}"));
    }
    if matches!(settings.device_policy.as_deref(), Some("strict" | "closed"))
        && !device_allow_contains_rw(&settings.device_allow, device)
    {
        failures.push(format!(
            "DevicePolicy={} without DeviceAllow={device} rw",
            settings.device_policy.as_deref().unwrap_or("strict")
        ));
    }

    if failures.is_empty() {
        DoctorCheck {
            name: CHECK_POLKIT_VIDEO_DEVICE_ACCESS.into(),
            status: CheckStatus::Pass,
            message: format!("{POLKIT_HELPER_UNIT} allows configured video device {device}"),
            path: None,
            device: Some(device.into()),
        }
    } else {
        DoctorCheck {
            name: CHECK_POLKIT_VIDEO_DEVICE_ACCESS.into(),
            status: CheckStatus::Fail,
            message: format!(
                "{}; polkit camera capture may fail. See {POLKIT_GUIDE}",
                failures.join("; ")
            ),
            path: None,
            device: Some(device.into()),
        }
    }
}

fn inspect_polkit_unit() -> Result<PolkitUnitSettings, String> {
    let show = Command::new("systemctl")
        .args([
            "show",
            POLKIT_HELPER_UNIT,
            "-p",
            "LoadState",
            "-p",
            "ProtectHome",
            "-p",
            "BindReadOnlyPaths",
            "-p",
            "BindPaths",
            "-p",
            "PrivateDevices",
            "-p",
            "DevicePolicy",
            "-p",
            "DeviceAllow",
            "--no-pager",
        ])
        .output();

    match show {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Ok(parse_systemctl_show(&stdout));
        }
        Ok(_) | Err(_) => {}
    }

    let cat = Command::new("systemctl")
        .args(["cat", POLKIT_HELPER_UNIT])
        .output()
        .map_err(|err| err.to_string())?;
    if !cat.status.success() {
        return Err(String::from_utf8_lossy(&cat.stderr).trim().to_string());
    }
    Ok(parse_systemctl_cat(&String::from_utf8_lossy(&cat.stdout)))
}

fn parse_systemctl_show(contents: &str) -> PolkitUnitSettings {
    let mut settings = PolkitUnitSettings {
        source: "systemctl show".into(),
        ..PolkitUnitSettings::default()
    };
    for line in contents.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        apply_polkit_setting(&mut settings, key.trim(), value.trim());
    }
    settings
}

fn polkit_unit_unavailable(settings: &PolkitUnitSettings) -> bool {
    matches!(settings.load_state.as_deref(), Some("not-found" | "masked"))
}

fn parse_systemctl_cat(contents: &str) -> PolkitUnitSettings {
    let mut settings = PolkitUnitSettings {
        source: "systemctl cat".into(),
        ..PolkitUnitSettings::default()
    };
    for line in contents.lines() {
        let active = line.split('#').next().map(str::trim).unwrap_or("");
        if active.is_empty() || active.starts_with('[') {
            continue;
        }
        let Some((key, value)) = active.split_once('=') else {
            continue;
        };
        apply_polkit_setting(&mut settings, key.trim(), value.trim());
    }
    settings
}

fn apply_polkit_setting(settings: &mut PolkitUnitSettings, key: &str, value: &str) {
    match key {
        "LoadState" => settings.load_state = non_empty(value),
        "ProtectHome" => settings.protect_home = non_empty(value),
        "BindReadOnlyPaths" => apply_systemd_list(&mut settings.bind_read_only_paths, value),
        "BindPaths" => apply_systemd_list(&mut settings.bind_paths, value),
        "PrivateDevices" => settings.private_devices = non_empty(value),
        "DevicePolicy" => settings.device_policy = non_empty(value),
        "DeviceAllow" => {
            if value.is_empty() {
                settings.device_allow.clear();
            } else {
                settings.device_allow.push(value.into());
            }
        }
        _ => {}
    }
}

fn non_empty(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.into())
    }
}

fn split_systemd_list(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn apply_systemd_list(target: &mut Vec<String>, value: &str) {
    if value.is_empty() {
        target.clear();
    } else {
        target.extend(split_systemd_list(value));
    }
}

fn contains_bound_path(entries: &[String], needle: &str) -> bool {
    entries.iter().any(|entry| {
        let source = entry.split(':').next().unwrap_or(entry);
        source == needle || needle.starts_with(&format!("{source}/"))
    })
}

fn device_allow_contains_rw(entries: &[String], device: &str) -> bool {
    entries.iter().any(|entry| {
        let parts = entry.split_whitespace().collect::<Vec<_>>();
        parts.iter().enumerate().any(|(index, part)| {
            *part == device
                && parts[index + 1..]
                    .iter()
                    .take_while(|candidate| !candidate.starts_with('/'))
                    .any(|candidate| candidate.contains('r') && candidate.contains('w'))
        })
    })
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

fn is_traversable_dir(path: &Path) -> bool {
    let c_path = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
        Ok(c) => c,
        Err(_) => return false,
    };
    unsafe { libc::access(c_path.as_ptr(), libc::X_OK) == 0 }
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

    #[derive(Clone)]
    struct StubPolkitInspector {
        result: Result<PolkitUnitSettings, String>,
    }

    impl PolkitInspector for StubPolkitInspector {
        fn inspect(&self) -> Result<PolkitUnitSettings, String> {
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
            secret_service_session: None,
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
            polkit_inspector: StubPolkitInspector {
                result: Err("not used".into()),
            },
            fallback_config: fallback,
        };
        run_doctor_with(&ctx).unwrap()
    }

    fn doctor_with_options(
        paths: DoctorPaths,
        probe: StubProbe,
        device_ok: bool,
        fallback: ResolvedConfig,
        options: DoctorOptions,
        polkit_result: Result<PolkitUnitSettings, String>,
    ) -> DoctorOutcome {
        let ctx = DoctorContext {
            paths,
            secret_service_probe: probe,
            device_opener: StubDeviceOpener { ok: device_ok },
            polkit_inspector: StubPolkitInspector {
                result: polkit_result,
            },
            fallback_config: fallback,
        };
        run_doctor_with_options_and_context(&ctx, options).unwrap()
    }

    fn status<'a>(checks: &'a [DoctorCheck], name: &str) -> &'a DoctorCheck {
        checks
            .iter()
            .find(|c| c.name == name)
            .expect("check present")
    }

    fn registry_names(options: DoctorOptions) -> Vec<&'static str> {
        check_registry::<StubProbe, StubDeviceOpener, StubPolkitInspector>()
            .into_iter()
            .filter(|spec| spec.enabled(options))
            .map(|spec| spec.name)
            .collect()
    }

    fn check_names(outcome: &DoctorOutcome) -> Vec<&str> {
        outcome
            .checks
            .iter()
            .map(|check| check.name.as_str())
            .collect()
    }

    #[test]
    fn registry_default_profile_excludes_polkit_checks() {
        assert_eq!(
            registry_names(DoctorOptions {
                include_polkit: false
            }),
            vec![
                CHECK_CONFIG,
                CHECK_VIDEO_DEVICE,
                CHECK_EMBEDDING_DIR,
                CHECK_LANDMARK_MODEL,
                CHECK_ENCODER_MODEL,
                CHECK_SECRET_SERVICE,
                CHECK_PAM_STACK,
                CHECK_PAM_MODULE,
            ]
        );
    }

    #[test]
    fn registry_polkit_profile_appends_polkit_checks() {
        assert_eq!(
            registry_names(DoctorOptions {
                include_polkit: true
            }),
            vec![
                CHECK_CONFIG,
                CHECK_VIDEO_DEVICE,
                CHECK_EMBEDDING_DIR,
                CHECK_LANDMARK_MODEL,
                CHECK_ENCODER_MODEL,
                CHECK_SECRET_SERVICE,
                CHECK_PAM_STACK,
                CHECK_PAM_MODULE,
                CHECK_POLKIT_HELPER_UNIT,
                CHECK_POLKIT_SESSION_BUS_ACCESS,
                CHECK_POLKIT_VIDEO_DEVICE_ACCESS,
            ]
        );
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
            check_names(&outcome),
            vec![
                CHECK_CONFIG,
                CHECK_VIDEO_DEVICE,
                CHECK_EMBEDDING_DIR,
                CHECK_LANDMARK_MODEL,
                CHECK_ENCODER_MODEL,
                CHECK_SECRET_SERVICE,
                CHECK_PAM_STACK,
                CHECK_PAM_MODULE,
            ]
        );
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

    #[test]
    fn pam_module_check_uses_module_path_discovered_by_pam_stack() {
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
        fs::remove_file(tmp.path().join("libpam_chissu.so")).unwrap();
        let referenced_module = tmp.path().join("custom/libpam_chissu.so");
        fs::create_dir_all(referenced_module.parent().unwrap()).unwrap();
        File::create(&referenced_module).unwrap();
        fs::set_permissions(&referenced_module, fs::Permissions::from_mode(0o644)).unwrap();
        fs::write(
            tmp.path().join("pam.d/login"),
            format!("auth sufficient {}\n", referenced_module.display()),
        )
        .unwrap();

        let outcome = doctor_with(
            base_paths(tmp.path()),
            StubProbe { result: Ok(()) },
            true,
            resolved_for(tmp.path()),
        );

        let check = status(&outcome.checks, CHECK_PAM_MODULE);
        assert_eq!(check.status, CheckStatus::Pass);
        assert_eq!(
            check.path.as_deref(),
            Some(referenced_module.to_str().unwrap())
        );
    }

    #[test]
    fn doctor_accepts_sticky_non_listable_embedding_dir() {
        let tmp = tempdir().unwrap();
        write_fixtures(tmp.path());
        fs::set_permissions(tmp.path().join("store"), fs::Permissions::from_mode(0o1733)).unwrap();
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
            status(&outcome.checks, CHECK_EMBEDDING_DIR).status,
            CheckStatus::Pass
        );
    }

    #[test]
    fn doctor_accepts_when_any_pam_module_candidate_is_valid() {
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

        let fallback_ok = tmp.path().join("security/libpam_chissu.so");
        fs::create_dir_all(fallback_ok.parent().unwrap()).unwrap();
        File::create(&fallback_ok).unwrap();
        fs::set_permissions(&fallback_ok, fs::Permissions::from_mode(0o644)).unwrap();

        let paths = DoctorPaths {
            config_paths: vec![tmp.path().join("config.toml")],
            pam_module_paths: vec![
                tmp.path().join("missing/libpam_chissu.so"),
                fallback_ok.clone(),
                tmp.path().join("missing2/libpam_chissu.so"),
            ],
            pamd_dir: tmp.path().join("pam.d"),
        };

        let outcome = doctor_with(
            paths,
            StubProbe { result: Ok(()) },
            true,
            resolved_for(tmp.path()),
        );

        assert_eq!(
            status(&outcome.checks, CHECK_PAM_MODULE).status,
            CheckStatus::Pass
        );
    }

    #[test]
    fn doctor_fails_when_referenced_module_permissions_are_insecure() {
        let tmp = tempdir().unwrap();
        write_fixtures(tmp.path());
        let module = tmp.path().join("libpam_chissu.so");
        fs::set_permissions(&module, fs::Permissions::from_mode(0o666)).unwrap();
        fs::write(
            tmp.path().join("pam.d/login"),
            format!("auth sufficient {}\n", module.display()),
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
            status(&outcome.checks, CHECK_PAM_MODULE).status,
            CheckStatus::Fail
        );
    }

    #[test]
    fn plain_doctor_does_not_include_polkit_checks() {
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

        assert!(outcome
            .checks
            .iter()
            .all(|check| !check.name.starts_with("polkit_")));
    }

    #[test]
    fn polkit_doctor_includes_polkit_checks() {
        let tmp = tempdir().unwrap();
        write_fixtures(tmp.path());
        fs::write(
            tmp.path().join("config.toml"),
            format!(
                "embedding_store_dir = \"{}\"\nvideo_device = \"/dev/video2\"\nlandmark_model = \"{}\"\nencoder_model = \"{}\"\n",
                tmp.path().join("store").display(),
                tmp.path().join("landmark.dat").display(),
                tmp.path().join("encoder.dat").display()
            ),
        )
        .unwrap();

        let outcome = doctor_with_options(
            base_paths(tmp.path()),
            StubProbe { result: Ok(()) },
            true,
            resolved_for(tmp.path()),
            DoctorOptions {
                include_polkit: true,
            },
            Ok(recommended_polkit_settings()),
        );

        assert_eq!(
            check_names(&outcome),
            vec![
                CHECK_CONFIG,
                CHECK_VIDEO_DEVICE,
                CHECK_EMBEDDING_DIR,
                CHECK_LANDMARK_MODEL,
                CHECK_ENCODER_MODEL,
                CHECK_SECRET_SERVICE,
                CHECK_PAM_STACK,
                CHECK_PAM_MODULE,
                CHECK_POLKIT_HELPER_UNIT,
                CHECK_POLKIT_SESSION_BUS_ACCESS,
                CHECK_POLKIT_VIDEO_DEVICE_ACCESS,
            ]
        );
        assert_eq!(
            status(&outcome.checks, CHECK_POLKIT_HELPER_UNIT).status,
            CheckStatus::Pass
        );
        assert_eq!(
            status(&outcome.checks, CHECK_POLKIT_SESSION_BUS_ACCESS).status,
            CheckStatus::Pass
        );
        assert_eq!(
            status(&outcome.checks, CHECK_POLKIT_VIDEO_DEVICE_ACCESS).status,
            CheckStatus::Pass
        );
    }

    #[test]
    fn strict_polkit_unit_fails_bus_and_camera_checks() {
        let settings = parse_systemctl_cat(
            r#"
[Service]
ProtectHome=yes
PrivateDevices=yes
DevicePolicy=strict
DeviceAllow=/dev/null rw
"#,
        );

        assert_eq!(
            check_polkit_session_bus_access(&settings).status,
            CheckStatus::Fail
        );
        assert_eq!(
            check_polkit_video_device_access(&settings, "/dev/video2").status,
            CheckStatus::Fail
        );
    }

    #[test]
    fn recommended_polkit_override_passes_bus_and_camera_checks() {
        let settings = recommended_polkit_settings();

        assert_eq!(
            check_polkit_session_bus_access(&settings).status,
            CheckStatus::Pass
        );
        assert_eq!(
            check_polkit_video_device_access(&settings, "/dev/video2").status,
            CheckStatus::Pass
        );
    }

    #[test]
    fn polkit_video_check_fails_when_camera_allow_is_missing() {
        let settings = parse_systemctl_show(
            r#"
LoadState=loaded
ProtectHome=tmpfs
BindReadOnlyPaths=/run/user
BindPaths=/dev/video2
PrivateDevices=yes
DevicePolicy=strict
DeviceAllow=/dev/null rw
"#,
        );

        let check = check_polkit_video_device_access(&settings, "/dev/video2");

        assert_eq!(check.status, CheckStatus::Fail);
        assert!(check.message.contains("DevicePolicy=strict"));
    }

    #[test]
    fn unavailable_polkit_unit_reports_warnings() {
        let tmp = tempdir().unwrap();
        write_fixtures(tmp.path());

        let outcome = doctor_with_options(
            base_paths(tmp.path()),
            StubProbe { result: Ok(()) },
            true,
            resolved_for(tmp.path()),
            DoctorOptions {
                include_polkit: true,
            },
            Err("systemctl unavailable".into()),
        );

        assert_eq!(
            status(&outcome.checks, CHECK_POLKIT_HELPER_UNIT).status,
            CheckStatus::Warn
        );
        assert_eq!(
            status(&outcome.checks, CHECK_POLKIT_SESSION_BUS_ACCESS).status,
            CheckStatus::Warn
        );
        assert_eq!(
            status(&outcome.checks, CHECK_POLKIT_VIDEO_DEVICE_ACCESS).status,
            CheckStatus::Warn
        );
    }

    #[test]
    fn missing_polkit_unit_skips_dependent_checks() {
        let settings = parse_systemctl_show("LoadState=not-found\n");

        assert_eq!(
            check_polkit_helper_unit(&settings).status,
            CheckStatus::Warn
        );
        assert_eq!(
            check_polkit_session_bus_access(&settings).status,
            CheckStatus::Warn
        );
        assert_eq!(
            check_polkit_video_device_access(&settings, "/dev/video2").status,
            CheckStatus::Warn
        );
    }

    fn recommended_polkit_settings() -> PolkitUnitSettings {
        parse_systemctl_cat(
            r#"
# /usr/lib/systemd/system/polkit-agent-helper@.service
[Service]
ProtectHome=yes
PrivateDevices=yes
DevicePolicy=strict
DeviceAllow=/dev/null rw

# /etc/systemd/system/polkit-agent-helper@.service.d/override.conf
[Service]
ProtectHome=tmpfs
BindReadOnlyPaths=/run/user
BindPaths=/dev/video2
DeviceAllow=/dev/video2 rw
"#,
        )
    }
}
