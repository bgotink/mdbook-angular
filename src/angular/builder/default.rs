use std::{fs, path::PathBuf};

use pathdiff::diff_paths;
use serde_json::{json, Value};

use crate::{angular::AngularWorkspace, ChapterWithCodeBlocks, Config, Error, Result};

use super::{ng_build, utils::PROJECT_NAME, utils::TARGET_NAME, Writer};

pub(super) const BUILDER_NAME: &str = "@angular/build:application";
pub(super) const MAIN_FILENAME: &str = "load-angular.ts";

pub(super) fn write_angular_workspace(
	config: &Config,
	root: &PathBuf,
	optimize: bool,
) -> Result<()> {
	let Some(output_path) = diff_paths(&config.target_folder, root) else {
		return Err(Error::msg("Failed to find relative target folder"));
	};

	let mut workspace = AngularWorkspace::new();

	workspace.add_project(PROJECT_NAME, "").add_target(
		TARGET_NAME,
		BUILDER_NAME,
		json!({
		  "progress": false,
			"deleteOutputPath": false,

		  "index": false,
		  "aot": true,
		  "tsConfig": "tsconfig.json",
		  "browser": MAIN_FILENAME,
			"inlineStyleLanguage": &config.inline_style_language,
			"outputPath": output_path,

			"statsJson": optimize,
			"optimization": if optimize {
				json!({
					"styles": {
						"inlineCritical": false,
						"minify": true,
					},
					"scripts": true,
				})
			} else {
				json!(false)
			},
			"outputHashing": if optimize { "all" } else { "none" },
		}),
	);

	workspace.write(root)?;
	Ok(())
}

pub(super) fn build(config: &Config, chapters: Vec<ChapterWithCodeBlocks>) -> Result<()> {
	let root = &config.angular_root_folder;
	let mut writer = Writer::new(false);

	if root.exists() {
		fs::remove_dir_all(root)?;
	}

	fs::create_dir_all(root)?;

	writer.write_tsconfig(config)?;

	write_angular_workspace(config, root, config.optimize)?;

	let mut chapter_paths: Vec<PathBuf> = Vec::with_capacity(chapters.len());

	for (
		index,
		ChapterWithCodeBlocks {
			source_path,
			code_blocks,
		},
	) in chapters.into_iter().enumerate()
	{
		writer.write_chapter(root, index, &source_path, code_blocks)?;
		chapter_paths.push(source_path);
	}

	writer.write_main(config, root)?;

	ng_build(root)?;

	if config.optimize {
		replace_load_angular_script_path(config, chapter_paths)?;
	}

	Ok(())
}

fn replace_load_angular_script_path(config: &Config, chapters: Vec<PathBuf>) -> Result<()> {
	let stats_path = config.target_folder.join("stats.json");
	let stats: Value = serde_json::from_str(&fs::read_to_string(&stats_path)?)?;

	fs::remove_file(stats_path)?;

	let output = stats
		.get("outputs")
		.and_then(Value::as_object)
		.ok_or_else(|| Error::msg("Failed to parse stats.json"))?;
	let (main_file, _) = output
		.iter()
		.find(|(_, output)| {
			matches!(
				output.get("entryPoint").and_then(Value::as_str),
				Some(MAIN_FILENAME)
			)
		})
		.ok_or_else(|| Error::msg("Failed to find main file in stats.json"))?;

	let main_file = format!("browser/{main_file}",);

	for chapter_path in chapters {
		let mut chapter_path = config.target_folder.join(chapter_path);
		chapter_path.set_extension("html");

		if let Ok(chapter) = fs::read_to_string(&chapter_path) {
			let chapter = chapter.replace("browser/main.js", &main_file);

			fs::write(&chapter_path, &chapter)?;
		}
	}

	let index_path = config.target_folder.join("index.html");
	if let Ok(index) = fs::read_to_string(&index_path) {
		let index = index.replace("browser/main.js", &main_file);

		fs::write(&index_path, &index)?;
	}

	Ok(())
}
