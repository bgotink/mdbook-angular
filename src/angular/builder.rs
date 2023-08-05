#[cfg(all(unix, feature = "background"))]
mod background;
mod default;
mod experimental;
mod utils;
mod writer;

use log::warn;
use mdbook::renderer::RenderContext;

use crate::{Builder, ChapterWithCodeBlocks, Config, Result};

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
	match config.builder {
		#[cfg(not(unix))]
		Builder::Background => {
			warn!("The background option is not supported on windows");
			self::experimental::build(config, chapters)
		}
		#[cfg(all(unix, not(feature = "background")))]
		Builder::Background => {
			warn!("This build doesn't support the background option");
			self::experimental::build(config, chapters)
		}
		#[cfg(all(unix, feature = "background"))]
		Builder::Background => {
			if ctx.config.get("output.html.live-reload-endpoint").is_some() {
				self::background::build(config, chapters)
			} else {
				warn!("The background option is ignored for commands that don't watch");
				self::experimental::build(config, chapters)
			}
		}
		Builder::Experimental => self::experimental::build(config, chapters),
		Builder::Slow => self::default::build(config, chapters),
	}
}
