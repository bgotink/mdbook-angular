mod builder;
mod workspace;

pub(crate) use builder::build;

use log::error;
use workspace::AngularWorkspace;

use crate::{Config, Result};

/// Stop any running background process for the given configuration
///
/// # Errors
///
/// This function will return an error if any errors occur trying to detect or
/// stop the process. This is only likely to happen when trying to stop a
/// background process not belonging to the current user.
pub fn stop_background_process(config: &Config) -> Result<()> {
	if cfg!(all(unix, feature = "background")) {
		builder::stop_background_process(config)
	} else {
		error!("Background process is not supported by this build");
		Ok(())
	}
}
