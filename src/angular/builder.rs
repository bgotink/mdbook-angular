#[cfg(all(unix, feature = "background"))]
mod background;
mod default;
mod utils;
mod writer;

use log::warn;
use mdbook::renderer::RenderContext;

use crate::{Builder, ChapterWithCodeBlocks, Config, Result};

use utils::ng_build;
use writer::Writer;

#[cfg(all(unix, feature = "background"))]
pub(crate) use background::stop_background_process;

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
			self::default::build(config, chapters)
		}
		#[cfg(all(unix, not(feature = "background")))]
		Builder::Background => {
			warn!("This build doesn't support the background option");
			self::default::build(config, chapters)
		}
		#[cfg(all(unix, feature = "background"))]
		Builder::Background => {
			if ctx.config.get("output.html.live-reload-endpoint").is_some() {
				self::background::build(config, chapters)
			} else {
				warn!("The background option is ignored for commands that don't watch");
				self::default::build(config, chapters)
			}
		}
		Builder::Experimental => {
			warn!(
				r#"The experimental builder is no longer experimental, switch to "foreground" instead"#
			);
			self::default::build(config, chapters)
		}
		Builder::Slow => {
			warn!(r#"The slow builder is no longer present, switch to "foreground" instead"#);
			self::default::build(config, chapters)
		}
		Builder::Foreground => self::default::build(config, chapters),
	}
}
