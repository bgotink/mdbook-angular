use std::{
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
	process::{Command, Stdio},
};

use anyhow::{Error, Result};
use pathdiff::diff_paths;
use serde_json::json;

use crate::{
	config::Config,
	js::{
		EXPERIMENTAL_BUILDER_IMPLEMENTATION, EXPERIMENTAL_BUILDER_MANIFEST,
		EXPERIMENTAL_BUILDER_SCHEMA,
	},
	markdown::ChapterWithCodeBlocks,
	utils::path_to_root,
};

use super::workspace::AngularWorkspace;

struct PostBuildReplacement {
	chapter_path: PathBuf,
	has_playgrounds: bool,
	project_folder: String,
	script_basename: String,
	script_marker: String,
}

pub(super) enum Writer {
	Default,
	#[allow(dead_code)]
	ChangedOnly,
}

impl Writer {
	fn write<P: AsRef<Path>>(&self, path: P, contents: &str) -> Result<bool> {
		if matches!(self, Self::ChangedOnly) {
			let existing = fs::read_to_string(&path)?;

			if existing.eq(contents) {
				return Ok(false);
			}
		}

		fs::write(&path, contents)?;

		Ok(true)
	}

	fn write_chapter<P: AsRef<Path>>(
		&self,
		root: P,
		index: usize,
		chapter: ChapterWithCodeBlocks,
	) -> Result<PostBuildReplacement> {
		let root = root.as_ref();
		let project_folder = format!("code_{index}");
		let script_marker = chapter.script_marker().to_owned();
		let has_playgrounds = chapter.has_playgrounds();

		let absolute_project_folder = root.join(&project_folder);

		if absolute_project_folder.exists() {
			if let Self::Default = self {
				fs::remove_dir_all(&absolute_project_folder)?;
			}
		}

		fs::create_dir_all(&absolute_project_folder)?;

		let mut main_script = Vec::with_capacity(1 + chapter.number_of_code_blocks());

		main_script.push(
			"\
				import 'zone.js';\n\
				import {NgZone, type ApplicationRef} from '@angular/core';\n\
				import {bootstrapApplication} from '@angular/platform-browser';\n\
				const zone = new NgZone({});\n\
				const providers = [{provide: NgZone, useValue: zone}];\n\
				const applications: Promise<ApplicationRef>[] = [];\n\
				(globalThis as any).mdBookAngular = {zone, applications};\n\
			"
			.to_owned(),
		);

		let chapter_path = chapter.source_path().to_owned();

		for (code_block_index, code_block) in chapter.into_iter().enumerate() {
			self.write(
				absolute_project_folder.join(&format!("codeblock_{code_block_index}.ts")),
				&code_block.code_to_run,
			)?;

			main_script.push(format!(
				"\
					import {{{} as CodeBlock_{code_block_index}}} from './codeblock_{code_block_index}.js';\n\
					applications.push(bootstrapApplication(CodeBlock_{code_block_index}, {{providers}}));\n\
				",
				&code_block.class_name
			));
		}

		let script_basename = project_folder.clone();

		let angular_main = format!("{}/{}.ts", &project_folder, &script_basename);
		self.write(root.join(angular_main), &main_script.join("\n"))?;

		Ok(PostBuildReplacement {
			chapter_path,
			has_playgrounds,
			project_folder,
			script_basename,
			script_marker,
		})
	}
}

#[allow(clippy::too_many_lines)]
fn build_default(
	config: &Config,
	writer: &Writer,
	chapters: Vec<ChapterWithCodeBlocks>,
) -> Result<()> {
	let mut workspace = AngularWorkspace::new();

	let root = &config.angular_root_folder;
	if root.exists() {
		fs::remove_dir_all(root)?;
	}

	fs::create_dir_all(root)?;
	write_tsconfig(writer, config)?;

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

		let replacement = writer.write_chapter(root, index, chapter)?;

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
				"build",
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

	writer.write(
		root.join("angular.json"),
		&serde_json::to_string(&workspace)?,
	)?;

	let mut has_playgrounds = false;
	let mut marker_to_script_map = HashMap::new();

	for (index, replacement) in replacements.into_iter().enumerate() {
		let project_name = format!("code_{index}");

		ng_build(root, &project_name)?;

		let mut chapter_path = config.target_folder.join(&replacement.chapter_path);
		chapter_path.set_extension("html");

		let chapter = fs::read_to_string(&chapter_path)?;
		let path_to_root = path_to_root(&replacement.chapter_path);

		let Some(main_filename) = fs::read_dir(root_target_folder.join(&replacement.project_folder))?
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
		marker_to_script_map.insert(
			replacement.script_marker,
			(main_filename, replacement.has_playgrounds),
		);

		if replacement.has_playgrounds {
			chapter.push_str(&format!(
				r#"<script type="module" src="{path_to_root}/playground-io.min.js"></script>"#
			));
			has_playgrounds = true;
		}

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

	if has_playgrounds {
		fs::write(
			config.target_folder.join("playground-io.min.js"),
			crate::js::PLAYGROUND_SCRIPT,
		)?;
	}

	Ok(())
}

fn build_experimental(
	config: &Config,
	writer: &Writer,
	chapters: Vec<ChapterWithCodeBlocks>,
) -> Result<()> {
	let root = &config.angular_root_folder;

	let mut root_exists = root.exists();
	if root_exists && matches!(writer, Writer::Default) {
		root_exists = false;
		fs::remove_dir_all(root)?;
	}

	if !root_exists {
		let experimental_builder_folder = root.join("node_modules/experimental-builder");
		fs::create_dir_all(&experimental_builder_folder)?;

		write_tsconfig(writer, config)?;

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
	}

	let Some(output_path) = diff_paths(&config.target_folder, root) else {
			return Err(Error::msg("Failed to find relative target folder"));
		};

	let mut workspace = AngularWorkspace::new();

	workspace.add_project("application", "").add_target(
		"build",
		"experimental-builder:application",
		json!({
			"optimization": config.optimize,
			"inlineStyleLanguage": &config.inline_style_language,
			"outputPath": output_path,
		}),
	);

	writer.write(
		root.join("angular.json"),
		&serde_json::to_string(&workspace)?,
	)?;

	let mut replacements = Vec::with_capacity(chapters.len());

	for (index, chapter) in chapters.into_iter().enumerate() {
		replacements.push(writer.write_chapter(root, index, chapter)?);
	}

	ng_build(root, "application")?;

	let scripts: HashMap<_, _> = fs::read_dir(&config.target_folder)?
		.filter_map(Result::ok)
		.filter_map(|entry| entry.file_name().to_str().map(ToOwned::to_owned))
		.filter_map(|name| name.find('.').map(|idx| (name[0..idx].to_owned(), name)))
		.collect();

	let mut has_playgrounds = false;
	let mut marker_to_script_map = HashMap::new();

	for replacement in replacements {
		let mut chapter_path = config.target_folder.join(&replacement.chapter_path);
		chapter_path.set_extension("html");

		let chapter = fs::read_to_string(&chapter_path)?;
		let path_to_root = path_to_root(&replacement.chapter_path);

		let Some(main_filename) = scripts.get(&replacement.script_basename) else {
				return Err(Error::msg(
					format!("Failed to find angular application for chapter {:?}", &replacement.chapter_path)
				));
			};

		let app_script_src = format!(r#"src="{path_to_root}/{main_filename}""#);

		let mut chapter = chapter.replace(&replacement.script_marker, &app_script_src);
		marker_to_script_map.insert(
			replacement.script_marker,
			(main_filename, replacement.has_playgrounds),
		);

		if replacement.has_playgrounds {
			chapter.push_str(&format!(
				r#"<script type="module" src="{path_to_root}/playground-io.min.js"></script>"#
			));
			has_playgrounds = true;
		}

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

	if has_playgrounds {
		fs::write(
			config.target_folder.join("playground-io.min.js"),
			crate::js::PLAYGROUND_SCRIPT,
		)?;
	}

	Ok(())
}

fn write_tsconfig(writer: &Writer, config: &Config) -> Result<()> {
	let tsconfig = if let Some(tsconfig) = &config.tsconfig {
		json!({"extends": tsconfig.to_string_lossy()})
	} else {
		json!({
			"compilerOptions": {
					"strict": true,
					"sourceMap": true,
					"experimentalDecorators": true,
					"moduleResolution": "node",
					"importHelpers": true,
					"target": "ES2022",
					"module": "ES2022",
					"useDefineForClassFields": false,
					"lib": ["ES2022", "dom"],
			},
		})
	};

	writer.write(
		config.angular_root_folder.join("tsconfig.json"),
		&serde_json::to_string(&tsconfig)?,
	)?;

	Ok(())
}

fn ng_build(root: &Path, project_name: &str) -> Result<()> {
	let result = Command::new("ng")
		.arg("build")
		.arg(project_name)
		.current_dir(root)
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.stdin(Stdio::null())
		.status()?;

	if result.success() {
		Ok(())
	} else {
		Err(Error::msg("Angular builder failed"))
	}
}

pub(crate) fn build(config: &Config, chapters: Vec<ChapterWithCodeBlocks>) -> Result<()> {
	if config.experimental_builder {
		build_experimental(config, &Writer::Default, chapters)
	} else {
		build_default(config, &Writer::Default, chapters)
	}
}
