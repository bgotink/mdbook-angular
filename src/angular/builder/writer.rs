use std::{
	fs,
	path::{Path, PathBuf},
};

use serde_json::json;

use crate::{ChapterWithCodeBlocks, Config, Context, Result};

pub(super) struct PostBuildReplacement {
	pub(super) chapter_path: PathBuf,
	pub(super) has_playgrounds: bool,
	pub(super) project_folder: String,
	pub(super) script_basename: String,
	pub(super) script_marker: String,
	#[cfg_attr(not(all(unix, feature = "background")), allow(dead_code))]
	pub(super) made_changes_to_scripts: bool,
}

pub(super) enum Writer {
	Default,
	#[cfg_attr(not(all(unix, feature = "background")), allow(dead_code))]
	ChangedOnly,
}

impl Writer {
	pub(super) fn write<P: AsRef<Path>>(&self, path: P, contents: &str) -> Result<bool> {
		if matches!(self, Self::ChangedOnly)
			&& matches!(fs::read_to_string(&path), Ok(existing) if existing.eq(contents))
		{
			return Ok(false);
		}

		fs::write(&path, contents)?;

		Ok(true)
	}

	pub(super) fn write_chapter<P: AsRef<Path>>(
		&self,
		config: &Config,
		root: P,
		index: usize,
		chapter: ChapterWithCodeBlocks,
	) -> Result<PostBuildReplacement> {
		let root = root.as_ref();
		let project_folder = format!("code_{index}");
		let script_marker = chapter.script_marker().to_owned();
		let has_playgrounds = chapter.has_playgrounds();

		let absolute_project_folder = root.join(&project_folder);
		let mut made_changes_to_scripts = false;

		if matches!(self, Self::Default) && absolute_project_folder.exists() {
			fs::remove_dir_all(&absolute_project_folder)
				.context("failed to clear project folder")?;
			made_changes_to_scripts = true;
		}

		fs::create_dir_all(&absolute_project_folder).context("failed to create project folder")?;

		let mut main_script =
			Vec::with_capacity(2 + config.polyfills.len() + chapter.number_of_code_blocks());

		if !config.polyfills.contains(&"zone.js".to_owned()) {
			main_script.push("import 'zone.js';".to_owned());
		}

		for polyfill in &config.polyfills {
			main_script.push(format!("import '{polyfill}';"));
		}

		main_script.push(
			"\n\
				import {NgZone, type ApplicationRef, type Provider, type EnvironmentProviders, type Type} from '@angular/core';\n\
				import {bootstrapApplication} from '@angular/platform-browser';\n\
				const zone = new NgZone({});\n\
				function makeProviders(component: Type<unknown> & {rootProviders?: readonly (Provider | EnvironmentProviders)[] | null | undefined}) {\n\
					return [{provide: NgZone, useValue: zone}, ...(component.rootProviders ?? [])];\n\
				}\n\
				const applications: Promise<ApplicationRef>[] = [];\n\
				(globalThis as any).mdBookAngular = {zone, applications};\n\
			"
			.to_owned(),
		);

		let chapter_path = chapter.source_path().to_owned();

		for (code_block_index, code_block) in chapter.into_iter().enumerate() {
			let changed_script = self
				.write(
					absolute_project_folder.join(&format!("codeblock_{code_block_index}.ts")),
					&code_block.code_to_run,
				)
				.context("failed to write code block")?;

			if changed_script && !made_changes_to_scripts {
				made_changes_to_scripts = changed_script;
			}

			main_script.push(format!(
				"\
					import {{{} as CodeBlock_{code_block_index}}} from './codeblock_{code_block_index}.js';\n\
					applications.push(bootstrapApplication(CodeBlock_{code_block_index}, {{providers: makeProviders(CodeBlock_{code_block_index})}}));\n\
				",
				&code_block.class_name
			));
		}

		let script_basename = project_folder.clone();

		let angular_main = format!("{}/{}.ts", &project_folder, &script_basename);
		self.write(root.join(angular_main), &main_script.join("\n"))
			.context("failed to write main chapter import")?;

		Ok(PostBuildReplacement {
			chapter_path,
			has_playgrounds,
			project_folder,
			script_basename,
			script_marker,
			made_changes_to_scripts,
		})
	}

	pub(super) fn write_tsconfig(&self, config: &Config) -> Result<()> {
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

		self.write(
			config.angular_root_folder.join("tsconfig.json"),
			&serde_json::to_string(&tsconfig)?,
		)
		.context("failed to write tsconfig.json")?;

		Ok(())
	}
}

pub(super) struct PlaygroundScriptWriter<'a> {
	config: &'a Config,
	has_playgrounds: bool,
}

impl<'a> PlaygroundScriptWriter<'a> {
	pub(super) fn new(config: &Config) -> PlaygroundScriptWriter<'_> {
		PlaygroundScriptWriter {
			config,
			has_playgrounds: false,
		}
	}

	pub(super) fn insert_playground_script(
		&mut self,
		replacement: &PostBuildReplacement,
		chapter: &mut String,
		path_to_root: &str,
	) {
		if replacement.has_playgrounds {
			chapter.push_str(&format!(
				r#"<script type="module" src="{path_to_root}/playground-io.min.js"></script>"#
			));
			self.has_playgrounds = true;
		}
	}

	pub(super) fn write_playground_file(&self) -> Result<()> {
		if self.has_playgrounds {
			fs::write(
				self.config.target_folder.join("playground-io.min.js"),
				crate::js::PLAYGROUND_SCRIPT,
			)?;
		}

		Ok(())
	}
}
