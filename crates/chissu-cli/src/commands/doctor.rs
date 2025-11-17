use std::any::Any;
use std::process::ExitCode;

use crate::cli::OutputMode;
use crate::commands::CommandHandler;
use crate::doctor::{self, DoctorOutcome};
use crate::errors::AppResult;
use crate::output::render_doctor;

pub struct DoctorHandler {
    run_doctor: Box<dyn Fn() -> AppResult<DoctorOutcome> + Send + Sync>,
    render: Box<dyn Fn(&DoctorOutcome, OutputMode) -> AppResult<()> + Send + Sync>,
}

impl DoctorHandler {
    pub fn new() -> Self {
        Self::with_dependencies(doctor::run_doctor, render_doctor)
    }

    pub fn with_dependencies(
        run_doctor: impl Fn() -> AppResult<DoctorOutcome> + Send + Sync + 'static,
        render: impl Fn(&DoctorOutcome, OutputMode) -> AppResult<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            run_doctor: Box::new(run_doctor),
            render: Box::new(render),
        }
    }
}

impl CommandHandler for DoctorHandler {
    fn execute(&self, mode: OutputMode, _verbose: bool) -> AppResult<ExitCode> {
        let outcome = (self.run_doctor)()?;
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
