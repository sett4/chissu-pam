use std::any::Any;
use std::process::ExitCode;

use crate::auto_enroll::{self, AutoEnrollOutcome};
use crate::cli::{EnrollArgs, OutputMode};
use crate::commands::CommandHandler;
use crate::errors::AppResult;
use crate::output::render_auto_enroll;

type EnrollRunner = dyn Fn(&EnrollArgs) -> AppResult<AutoEnrollOutcome> + Send + Sync;
type EnrollRenderer = dyn Fn(&AutoEnrollOutcome, OutputMode, bool) -> AppResult<()> + Send + Sync;

pub struct EnrollHandler {
    args: EnrollArgs,
    run: Box<EnrollRunner>,
    render: Box<EnrollRenderer>,
}

impl EnrollHandler {
    pub fn new(args: EnrollArgs) -> Self {
        Self::with_dependencies(args, auto_enroll::run_auto_enroll, render_auto_enroll)
    }

    pub fn with_dependencies(
        args: EnrollArgs,
        run: impl Fn(&EnrollArgs) -> AppResult<AutoEnrollOutcome> + Send + Sync + 'static,
        render: impl Fn(&AutoEnrollOutcome, OutputMode, bool) -> AppResult<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            args,
            run: Box::new(run),
            render: Box::new(render),
        }
    }
}

impl CommandHandler for EnrollHandler {
    fn execute(&self, mode: OutputMode, verbose: bool) -> AppResult<ExitCode> {
        let outcome = (self.run)(&self.args)?;
        (self.render)(&outcome, mode, verbose)?;
        Ok(ExitCode::SUCCESS)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
