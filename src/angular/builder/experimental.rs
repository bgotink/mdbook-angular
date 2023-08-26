use std::{collections::HashMap, fs, path::Path};

use pathdiff::diff_paths;
use serde_json::json;

use crate::{
	angular::AngularWorkspace,
	js::{
		EXPERIMENTAL_BUILDER_IMPLEMENTATION, EXPERIMENTAL_BUILDER_MANIFEST,
		EXPERIMENTAL_BUILDER_SCHEMA,
	},
	utils::path_to_root,
	ChapterWithCodeBlocks, Config, Error, Result,
};

use super::{ng_build, utils::TARGET_NAME, writer::PlaygroundScriptWriter, Writer};

pub(super) const PROJECT_NAME: &str = "application";
pub(super) const BUILDER_NAME: &str = "experimental-builder:application";

pub(super) fn build(config: &Config, chapters: Vec<ChapterWithCodeBlocks>) -> Result<()> {
	let root = &config.angular_root_folder;
	let writer = Writer::Default;

	if root.exists() {
		fs::remove_dir_all(root)?;
	}

	write_experimental_builder(root)?;

	writer.write_tsconfig(config)?;

	let Some(output_path) = diff_paths(&config.target_folder, root) else {
		return Err(Error::msg("Failed to find relative target folder"));
	};

	let mut workspace = AngularWorkspace::new();

	workspace.add_project(PROJECT_NAME, "").add_target(
		TARGET_NAME,
		BUILDER_NAME,
		json!({
			"optimization": config.optimize,
			"inlineStyleLanguage": &config.inline_style_language,
			"outputPath": output_path,
		}),
	);

	workspace.write(root)?;

	let mut replacements = Vec::with_capacity(chapters.len());

	for (index, chapter) in chapters.into_iter().enumerate() {
		replacements.push(writer.write_chapter(config, root, index, chapter)?);
	}

	ng_build(root, PROJECT_NAME)?;

	let scripts: HashMap<_, _> = fs::read_dir(&config.target_folder)?
		.filter_map(Result::ok)
		.filter_map(|entry| entry.file_name().to_str().map(ToOwned::to_owned))
		.filter_map(|name| name.find('.').map(|idx| (name[0..idx].to_owned(), name)))
		.collect();

	run_replacements(replacements, config, &scripts)?;

	Ok(())
}

pub(super) fn write_experimental_builder(root: &Path) -> Result<()> {
	let experimental_builder_folder = root.join("node_modules/experimental-builder");
	fs::create_dir_all(&experimental_builder_folder)?;

	fs::write(
		experimental_builder_folder.join("package.json"),
		EXPERIMENTAL_BUILDER_MANIFEST,
	)?;
	fs::write(
		experimental_builder_folder.join("builder.mjs"),
		EXPERIMENTAL_BUILDER_IMPLEMENTATION,
	)?;
	fs::write(
		experimental_builder_folder.join("schema.json"),
		EXPERIMENTAL_BUILDER_SCHEMA,
	)?;

	Ok(())
}

pub(super) fn run_replacements(
	replacements: Vec<super::writer::PostBuildReplacement>,
	config: &Config,
	scripts: &HashMap<String, String>,
) -> Result<()> {
	let mut marker_to_script_map = HashMap::new();
	let mut playground_writer = PlaygroundScriptWriter::new(config);

	for replacement in replacements {
		let mut chapter_path = config.target_folder.join(&replacement.chapter_path);
		chapter_path.set_extension("html");

		let chapter = fs::read_to_string(&chapter_path)?;
		let path_to_root = path_to_root(&replacement.chapter_path);

		let Some(main_filename) = scripts.get(&replacement.script_basename) else {
			return Err(Error::msg(format!(
				"Failed to find angular application for chapter {:?}",
				&replacement.chapter_path
			)));
		};

		let app_script_src = format!(r#"src="{path_to_root}/{main_filename}""#);

		let mut chapter = chapter.replace(&replacement.script_marker, &app_script_src);
		playground_writer.insert_playground_script(&replacement, &mut chapter, &path_to_root);

		marker_to_script_map.insert(
			replacement.script_marker,
			(main_filename, replacement.has_playgrounds),
		);

		fs::write(&chapter_path, &chapter)?;
	}

	let index_path = config.target_folder.join("index.html");
	if let Ok(index) = fs::read_to_string(&index_path) {
		for (marker, (main_filename, has_playgrounds)) in marker_to_script_map {
			if !index.contains(&marker) {
				continue;
			}

			let app_script_src = format!(r#"src="{main_filename}""#);

			let mut index = index.replace(&marker, &app_script_src);

			if has_playgrounds {
				index.push_str(r#"<script type="module" src="playground-io.min.js"></script>"#);
			}

			fs::write(index_path, index)?;

			break;
		}
	}

	playground_writer.write_playground_file()?;

	Ok(())
}
