mod utils;

use std::{collections::HashMap, fs};

use filetime::{set_file_mtime, FileTime};
use pathdiff::diff_paths;
use serde_json::json;
use utils as background;

pub(crate) use utils::stop as stop_background_process;

use crate::{angular::AngularWorkspace, ChapterWithCodeBlocks, Config, Error, Result};

use super::{
	experimental::{run_replacements, write_experimental_builder, BUILDER_NAME, PROJECT_NAME},
	utils::TARGET_NAME,
	Writer,
};

pub(super) fn build(config: &Config, chapters: Vec<ChapterWithCodeBlocks>) -> Result<()> {
	let root = &config.angular_root_folder;
	let writer = Writer::ChangedOnly;

	let mut root_exists = root.exists();
	if root_exists && !background::is_running(config)? {
		root_exists = false;
		fs::remove_dir_all(root)?;
	}

	let chapter_count_file = root.join(".number-of-files");
	let new_chapter_count = chapters.len();
	let mut is_running = false;
	if root_exists {
		let running_chapter_count = fs::read_to_string(&chapter_count_file)
			.ok()
			.and_then(|s| s.trim().parse::<usize>().ok());

		if matches!(
			running_chapter_count,
			Some(count) if count == new_chapter_count
		) {
			is_running = background::is_running(config)?;
		} else {
			background::stop(config)?;
			fs::remove_dir_all(root)?;

			is_running = false;
			root_exists = false;
		}
	}

	if !root_exists {
		write_experimental_builder(root)?;

		Writer::Default.write_tsconfig(config)?;

		let Some(output_path) = diff_paths(&config.target_folder, root) else {
			return Err(Error::msg("Failed to find relative target folder"));
		};

		let mut workspace = AngularWorkspace::new();

		workspace.add_project(PROJECT_NAME, "").add_target(
			TARGET_NAME,
			BUILDER_NAME,
			json!({
				"optimization": false,
				"inlineStyleLanguage": &config.inline_style_language,
				"outputPath": output_path,
			}),
		);

		workspace.write(root)?;
	}

	let mut replacements = Vec::with_capacity(chapters.len());

	for (index, chapter) in chapters.into_iter().enumerate() {
		replacements.push(writer.write_chapter(config, root, index, chapter)?);
	}

	if is_running {
		if !replacements.is_empty()
			&& !replacements
				.iter()
				.any(|replacement| replacement.made_changes_to_scripts)
		{
			// change one watched file to trigger a new build, as the HTML renderer
			// has just wiped the target folder
			set_file_mtime(root.join("code_0/code_0.ts"), FileTime::now())?;
		}
	} else {
		background::start(config)?;
		fs::write(chapter_count_file, format!("{new_chapter_count}\n"))?;
	}

	let scripts: HashMap<_, _> = replacements
		.iter()
		.map(|replacement| {
			(
				replacement.script_basename.clone(),
				format!("browser/{}.js", &replacement.script_basename),
			)
		})
		.collect();

	run_replacements(replacements, config, &scripts)?;

	Ok(())
}
