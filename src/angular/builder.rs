#[cfg(all(unix, feature = "background"))]
mod background;
mod default;
mod experimental;
mod utils;
mod writer;

use log::warn;
use mdbook::renderer::RenderContext;

use crate::{ChapterWithCodeBlocks, Config, Result};

use utils::ng_build;
use writer::Writer;

#[cfg(all(unix, feature = "background"))]
pub(crate) use background::stop_background_process;

#[allow(clippy::same_functions_in_if_condition)]
pub(crate) fn build(
	#[cfg_attr(not(all(unix, feature = "background")), allow(unused_variables))]
	ctx: &RenderContext,
	config: &Config,
	chapters: Vec<ChapterWithCodeBlocks>,
) -> Result<()> {
	if config.background {
		#[cfg(not(unix))]
		warn!("The background option is not supported on windows");

		#[cfg(not(feature = "background"))]
		warn!("This build doesn't support the background option");

		#[cfg(all(unix, feature = "background"))]
		{
			if !config.experimental_builder {
				warn!("The background option requires the experimentalBuilder option");
			} else if ctx.config.get("output.html.live-reload-endpoint").is_none() {
				warn!("The background option is ignored for commands that don't watch");
			} else {
				return self::background::build(config, chapters);
			}
		}
	}

	if config.experimental_builder {
		self::experimental::build(config, chapters)
	} else {
		self::default::build(config, chapters)
	}
}
