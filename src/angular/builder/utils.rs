use std::{
	path::Path,
	process::{Command, Stdio},
};

use crate::{Error, Result};

pub(super) const ANGULAR_CLI_CMD: &str = "ng";
pub(super) const TARGET_NAME: &str = "build";

pub(super) fn ng_build(root: &Path, project_name: &str) -> Result<()> {
	let result = Command::new(ANGULAR_CLI_CMD)
		.arg(TARGET_NAME)
		.arg(project_name)
		.current_dir(root)
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.stdin(Stdio::null())
		.status()?;

	if result.success() {
		Ok(())
	} else {
		Err(Error::msg("Angular builder failed"))
	}
}
