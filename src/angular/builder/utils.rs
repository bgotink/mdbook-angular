use std::{
	path::Path,
	process::{Command, Stdio},
};

use anyhow::Context;

use crate::{Error, Result};

pub(super) const ANGULAR_CLI_CMD: &str = "ng";
pub(super) const TARGET_NAME: &str = "build";
pub(super) const PROJECT_NAME: &str = "application";

pub(super) fn ng_build(root: &Path) -> Result<()> {
	let result = Command::new(ANGULAR_CLI_CMD)
		.arg(TARGET_NAME)
		.arg(PROJECT_NAME)
		.current_dir(root)
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.stdin(Stdio::null())
		.status()
		.context("Failed to run ng")?;

	if result.success() {
		Ok(())
	} else {
		Err(Error::msg("Angular builder failed"))
	}
}
