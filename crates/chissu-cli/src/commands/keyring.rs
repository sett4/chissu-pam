use std::any::Any;
use std::process::ExitCode;

use crate::cli::{KeyringCommands, OutputMode};
use crate::commands::CommandHandler;
use crate::errors::AppResult;
use crate::keyring::{self, KeyringCheckSummary};
use crate::output::render_keyring_check;

pub struct KeyringHandler {
    command: KeyringCommands,
    check: Box<dyn Fn(&KeyringCommands) -> AppResult<KeyringCheckSummary> + Send + Sync>,
    render: Box<dyn Fn(&KeyringCheckSummary, OutputMode) -> AppResult<()> + Send + Sync>,
}

impl KeyringHandler {
    pub fn new(command: KeyringCommands) -> Self {
        Self::with_dependencies(command, default_check, render_keyring_check)
    }

    pub fn with_dependencies(
        command: KeyringCommands,
        check: impl Fn(&KeyringCommands) -> AppResult<KeyringCheckSummary> + Send + Sync + 'static,
        render: impl Fn(&KeyringCheckSummary, OutputMode) -> AppResult<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            command,
            check: Box::new(check),
            render: Box::new(render),
        }
    }
}

impl CommandHandler for KeyringHandler {
    fn execute(&self, mode: OutputMode, _verbose: bool) -> AppResult<ExitCode> {
        let summary = (self.check)(&self.command)?;
        (self.render)(&summary, mode)?;
        Ok(ExitCode::SUCCESS)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn default_check(command: &KeyringCommands) -> AppResult<KeyringCheckSummary> {
    match command {
        KeyringCommands::Check(_) => keyring::run_keyring_check(),
    }
}
