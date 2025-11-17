use std::any::Any;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::{FaceRemoveArgs, FacesCommands, OutputMode};
use crate::commands::CommandHandler;
use crate::config;
use crate::errors::AppResult;
use crate::faces::{
    self, FaceComparisonConfig, FaceComparisonOutcome, FaceEnrollmentConfig, FaceEnrollmentOutcome,
    FaceExtractionConfig, FaceExtractionOutcome, FaceRemovalConfig, FaceRemovalOutcome,
};
use crate::output::{
    render_face_compare, render_face_enroll, render_face_remove, render_face_success,
};

type ResolveStoreDirFn = dyn Fn(Option<PathBuf>) -> AppResult<Option<PathBuf>> + Send + Sync;
type FaceExtractRunner =
    dyn Fn(&FaceExtractionConfig) -> AppResult<FaceExtractionOutcome> + Send + Sync;
type FaceCompareRunner =
    dyn Fn(&FaceComparisonConfig) -> AppResult<FaceComparisonOutcome> + Send + Sync;
type FaceEnrollRunner =
    dyn Fn(&FaceEnrollmentConfig) -> AppResult<FaceEnrollmentOutcome> + Send + Sync;
type FaceRemoveRunner = dyn Fn(&FaceRemovalConfig) -> AppResult<FaceRemovalOutcome> + Send + Sync;
type FaceExtractRenderer =
    dyn Fn(&FaceExtractionOutcome, OutputMode) -> AppResult<()> + Send + Sync;
type FaceCompareRenderer =
    dyn Fn(&FaceComparisonOutcome, OutputMode) -> AppResult<()> + Send + Sync;
type FaceEnrollRenderer = dyn Fn(&FaceEnrollmentOutcome, OutputMode) -> AppResult<()> + Send + Sync;
type FaceRemoveRenderer = dyn Fn(&FaceRemovalOutcome, OutputMode) -> AppResult<()> + Send + Sync;

pub struct FacesHandler {
    command: FacesCommands,
    deps: FacesHandlerDeps,
}

pub struct FacesHandlerDeps {
    pub resolve_store_dir: Box<ResolveStoreDirFn>,
    pub extract: Box<FaceExtractRunner>,
    pub compare: Box<FaceCompareRunner>,
    pub enroll: Box<FaceEnrollRunner>,
    pub remove: Box<FaceRemoveRunner>,
    pub render_extract: Box<FaceExtractRenderer>,
    pub render_compare: Box<FaceCompareRenderer>,
    pub render_enroll: Box<FaceEnrollRenderer>,
    pub render_remove: Box<FaceRemoveRenderer>,
}

impl FacesHandlerDeps {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        resolve_store_dir: impl Fn(Option<PathBuf>) -> AppResult<Option<PathBuf>>
            + Send
            + Sync
            + 'static,
        extract: impl Fn(&FaceExtractionConfig) -> AppResult<FaceExtractionOutcome>
            + Send
            + Sync
            + 'static,
        compare: impl Fn(&FaceComparisonConfig) -> AppResult<FaceComparisonOutcome>
            + Send
            + Sync
            + 'static,
        enroll: impl Fn(&FaceEnrollmentConfig) -> AppResult<FaceEnrollmentOutcome>
            + Send
            + Sync
            + 'static,
        remove: impl Fn(&FaceRemovalConfig) -> AppResult<FaceRemovalOutcome> + Send + Sync + 'static,
        render_extract: impl Fn(&FaceExtractionOutcome, OutputMode) -> AppResult<()>
            + Send
            + Sync
            + 'static,
        render_compare: impl Fn(&FaceComparisonOutcome, OutputMode) -> AppResult<()>
            + Send
            + Sync
            + 'static,
        render_enroll: impl Fn(&FaceEnrollmentOutcome, OutputMode) -> AppResult<()>
            + Send
            + Sync
            + 'static,
        render_remove: impl Fn(&FaceRemovalOutcome, OutputMode) -> AppResult<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            resolve_store_dir: Box::new(resolve_store_dir),
            extract: Box::new(extract),
            compare: Box::new(compare),
            enroll: Box::new(enroll),
            remove: Box::new(remove),
            render_extract: Box::new(render_extract),
            render_compare: Box::new(render_compare),
            render_enroll: Box::new(render_enroll),
            render_remove: Box::new(render_remove),
        }
    }
}

impl Default for FacesHandlerDeps {
    fn default() -> Self {
        Self::new(
            config::resolve_store_dir,
            faces::run_face_extraction,
            faces::run_face_comparison,
            faces::run_face_enrollment,
            faces::run_face_removal,
            render_face_success,
            render_face_compare,
            render_face_enroll,
            render_face_remove,
        )
    }
}

impl FacesHandler {
    pub fn new(command: FacesCommands) -> Self {
        Self {
            command,
            deps: FacesHandlerDeps::default(),
        }
    }

    pub fn with_dependencies(command: FacesCommands, deps: FacesHandlerDeps) -> Self {
        Self { command, deps }
    }
}

impl CommandHandler for FacesHandler {
    fn execute(&self, mode: OutputMode, _verbose: bool) -> AppResult<ExitCode> {
        match &self.command {
            FacesCommands::Extract(args) => {
                let config = FaceExtractionConfig::from(args);
                let outcome = (self.deps.extract)(&config)?;
                (self.deps.render_extract)(&outcome, mode)?;
            }
            FacesCommands::Compare(args) => {
                let config = FaceComparisonConfig::from(args);
                let outcome = (self.deps.compare)(&config)?;
                (self.deps.render_compare)(&outcome, mode)?;
            }
            FacesCommands::Enroll(args) => {
                let store_dir = (self.deps.resolve_store_dir)(args.store_dir.clone())?;
                let config = FaceEnrollmentConfig {
                    user: args.user.clone(),
                    embedding: args.embedding.clone(),
                    store_dir,
                };
                let outcome = (self.deps.enroll)(&config)?;
                (self.deps.render_enroll)(&outcome, mode)?;
            }
            FacesCommands::Remove(args) => {
                let config = build_removal_config(args, &self.deps)?;
                let outcome = (self.deps.remove)(&config)?;
                (self.deps.render_remove)(&outcome, mode)?;
            }
        }
        Ok(ExitCode::SUCCESS)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn build_removal_config(
    args: &FaceRemoveArgs,
    deps: &FacesHandlerDeps,
) -> AppResult<FaceRemovalConfig> {
    let store_dir = (deps.resolve_store_dir)(args.store_dir.clone())?;
    Ok(FaceRemovalConfig {
        user: args.user.clone(),
        embedding_ids: args.embedding_id.clone(),
        remove_all: args.all,
        store_dir,
    })
}
