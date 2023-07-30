mod builder;
mod workspace;

pub(crate) use builder::build;

use workspace::AngularWorkspace;

use crate::{Config, Result};

/// Stop any running background process for the given configuration
///
/// # Errors
///
/// This function will return an error if any errors occur trying to detect or
/// stop the process. This is only likely to happen when trying to stop a
/// background process not belonging to the current user.
#[cfg_attr(not(all(unix, feature = "background")), allow(unused_variables))]
pub fn stop_background_process(config: &Config) -> Result<()> {
	#[cfg(not(unix))]
	{
		log::warn!("The background option is not supported on windows");
		Ok(())
	}

	#[cfg(not(feature = "background"))]
	{
		log::warn!("This build doesn't support the background option");
		Ok(())
	}

	#[cfg(all(unix, feature = "background"))]
	return builder::stop_background_process(config);
}
