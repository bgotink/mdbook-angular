use std::{collections::HashMap, fs};

use pathdiff::diff_paths;
use serde_json::json;

use crate::{
	angular::AngularWorkspace, utils::path_to_root, ChapterWithCodeBlocks, Config, Error, Result,
};

use super::{ng_build, utils::TARGET_NAME, writer::PlaygroundScriptWriter, Writer};

pub(super) fn build(config: &Config, chapters: Vec<ChapterWithCodeBlocks>) -> Result<()> {
	let mut workspace = AngularWorkspace::new();
	let writer = Writer::Default;

	let root = &config.angular_root_folder;
	if root.exists() {
		fs::remove_dir_all(root)?;
	}

	fs::create_dir_all(root)?;
	writer.write_tsconfig(config)?;

	let Some(root_target_folder) = diff_paths(&config.target_folder, root) else {
			return Err(Error::msg("Failed to find relative target folder"));
		};

	let mut replacements = Vec::with_capacity(chapters.len());

	let (optimization, output_hashing) = if config.optimize {
		(
			json!({
				"scripts": true,
				"styles": {
					"minify": true,
					"inlineCritical": false
				},
				"fonts": false
			}),
			json!("all"),
		)
	} else {
		(json!(false), json!("none"))
	};

	for (index, chapter) in chapters.into_iter().enumerate() {
		let project_name = format!("code_{index}");

		let replacement = writer.write_chapter(config, root, index, chapter)?;

		writer.write(
			root.join(&replacement.project_folder).join("tsconfig.json"),
			r#"{"extends":"../tsconfig.json"}"#,
		)?;

		writer.write(
			root.join(&replacement.project_folder).join("index.html"),
			r#"<!doctype html><html><body></body></html>"#,
		)?;

		workspace
			.add_project(&project_name, &replacement.project_folder)
			.add_target(
				TARGET_NAME,
				"@angular-devkit/build-angular:browser-esbuild",
				json!({
					"commonChunk": false,
					"index": format!("{}/index.html", &replacement.project_folder),
					"inlineStyleLanguage": &config.inline_style_language,
					"main": format!("{}/{}.ts", &replacement.project_folder, replacement.script_basename),
					"optimization": &optimization,
					"outputHashing": &output_hashing,
					"progress": false,
					"tsConfig": format!("{}/tsconfig.json", &replacement.project_folder),
					"outputPath": root_target_folder.join(&replacement.project_folder).as_os_str().to_string_lossy()
				}),
			);

		replacements.push(replacement);
	}

	workspace.write(root)?;

	run_replacements(replacements, config)?;

	Ok(())
}

fn run_replacements(
	replacements: Vec<super::writer::PostBuildReplacement>,
	config: &Config,
) -> Result<()> {
	let mut marker_to_script_map = HashMap::new();
	let mut playground_writer = PlaygroundScriptWriter::new(config);

	for (index, replacement) in replacements.into_iter().enumerate() {
		let project_name = format!("code_{index}");

		ng_build(&config.angular_root_folder, &project_name)?;

		let mut chapter_path = config.target_folder.join(&replacement.chapter_path);
		chapter_path.set_extension("html");

		let chapter = fs::read_to_string(&chapter_path)?;
		let path_to_root = path_to_root(&replacement.chapter_path);

		let Some(main_filename) = fs::read_dir(config.target_folder.join(&replacement.project_folder))?
				.filter_map(Result::ok)
				.find(|entry| {
					let file_name = entry.file_name();
					let file_name = file_name.to_string_lossy();
					file_name.ends_with(".js") && file_name.starts_with("main.")
				}) else {
					return Err(Error::msg(
						format!("Failed to find angular application for chapter {:?}", replacement.chapter_path)
					));
				};

		let main_filename = format!(
			"{}/{}",
			replacement.project_folder,
			main_filename.file_name().to_string_lossy()
		);

		let app_script_src = format!(r#"src="{}/{}""#, path_to_root, &main_filename);

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
