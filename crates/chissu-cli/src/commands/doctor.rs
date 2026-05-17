use std::any::Any;
use std::process::ExitCode;

use crate::cli::{DoctorArgs, OutputMode};
use crate::commands::CommandHandler;
use crate::doctor::{self, DoctorOptions, DoctorOutcome};
use crate::errors::AppResult;
use crate::output::render_doctor;

type DoctorRunner = dyn Fn(DoctorOptions) -> AppResult<DoctorOutcome> + Send + Sync;
type DoctorRenderer = dyn Fn(&DoctorOutcome, OutputMode) -> AppResult<()> + Send + Sync;

pub struct DoctorHandler {
    args: DoctorArgs,
    run_doctor: Box<DoctorRunner>,
    render: Box<DoctorRenderer>,
}

impl DoctorHandler {
    pub fn new(args: DoctorArgs) -> Self {
        Self::with_dependencies(args, doctor::run_doctor_with_options, render_doctor)
    }

    pub fn with_dependencies(
        args: DoctorArgs,
        run_doctor: impl Fn(DoctorOptions) -> AppResult<DoctorOutcome> + Send + Sync + 'static,
        render: impl Fn(&DoctorOutcome, OutputMode) -> AppResult<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            args,
            run_doctor: Box::new(run_doctor),
            render: Box::new(render),
        }
    }
}

impl Default for DoctorHandler {
    fn default() -> Self {
        Self::new(DoctorArgs { polkit: false })
    }
}

impl CommandHandler for DoctorHandler {
    fn execute(&self, mode: OutputMode, _verbose: bool) -> AppResult<ExitCode> {
        let outcome = (self.run_doctor)(DoctorOptions {
            include_polkit: self.args.polkit,
        })?;
        (self.render)(&outcome, mode)?;
        let exit = if outcome.ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
        Ok(exit)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
