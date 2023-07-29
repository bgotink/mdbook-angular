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
	ctx: &RenderContext,
	config: &Config,
	chapters: Vec<ChapterWithCodeBlocks>,
) -> Result<()> {
	if config.background {
		if !cfg!(unix) {
			warn!("The background option is not supported on windows");
		} else if !cfg!(feature = "background") {
			warn!("This build doesn't support the background option");
		} else if !config.experimental_builder {
			warn!("The background option requires the experimentalBuilder option");
		} else if ctx.config.get("output.html.live-reload-endpoint").is_none() {
			warn!("The background option is ignored for commands that don't watch");
		} else {
			return self::background::build(config, chapters);
		}
	}

	if config.experimental_builder {
		self::experimental::build(config, chapters)
	} else {
		self::default::build(config, chapters)
	}
}
