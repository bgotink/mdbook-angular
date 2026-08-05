mod utils;

use std::fs;
use utils as background;

pub(crate) use utils::stop as stop_background_process;

use crate::{ChapterWithCodeBlocks, Config, Result};

use super::{Writer, default::write_angular_workspace};

pub(super) fn build(config: &Config, chapters: Vec<ChapterWithCodeBlocks>) -> Result<()> {
	let root = &config.angular_root_folder;

	let mut root_exists = root.exists();
	if root_exists && !background::is_running(config)? {
		root_exists = false;
		fs::remove_dir_all(root)?;
	}

	let mut is_running = false;
	if root_exists {
		is_running = background::is_running(config)?;
	} else {
		fs::create_dir_all(root)?;
	}

	let mut writer = Writer::new(config, true);

	for (
		index,
		ChapterWithCodeBlocks {
			source_path,
			code_blocks,
		},
	) in chapters.into_iter().enumerate()
	{
		writer.write_chapter(root, index, &source_path, code_blocks)?;
	}

	writer.write_main(root)?;

	if !is_running {
		writer.write_tsconfig()?;

		write_angular_workspace(config, root, false)?;
	}

	if !is_running {
		background::start(config)?;
	}

	Ok(())
}
