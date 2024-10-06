use std::{fs, path::Path};

use serde_json::json;

use crate::{codeblock::CodeBlock, Config, Context, Result};

pub(super) struct Writer {
	changed_only: bool,
	chapter_to_angular_file: Vec<(String, String)>,
}

impl Writer {
	pub(super) fn new(changed_only: bool) -> Self {
		Self {
			changed_only,
			chapter_to_angular_file: Vec::new(),
		}
	}

	pub(super) fn write<P: AsRef<Path>>(&self, path: P, contents: &str) -> Result<()> {
		if self.changed_only
			&& matches!(fs::read_to_string(&path), Ok(existing) if existing.eq(contents))
		{
			return Ok(());
		}

		fs::write(&path, contents)?;

		Ok(())
	}

	#[allow(clippy::unused_self)]
	pub(super) fn write_force<P: AsRef<Path>>(&self, path: P, contents: &str) -> Result<()> {
		fs::write(&path, contents)?;

		Ok(())
	}

	pub(super) fn write_chapter<P: AsRef<Path>>(
		&mut self,
		root: P,
		index: usize,
		chapter_path: &Path,
		code_blocks: Vec<CodeBlock>,
	) -> Result<()> {
		let root = root.as_ref();
		let project_folder = format!("code_{index}");

		let absolute_project_folder = root.join(&project_folder);

		fs::create_dir_all(&absolute_project_folder).context("failed to create project folder")?;

		let mut main_script = Vec::with_capacity(1 + code_blocks.len());

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

		for (code_block_index, code_block) in code_blocks.into_iter().enumerate() {
			self.write(
				absolute_project_folder.join(format!("codeblock_{code_block_index}.ts")),
				&code_block.code_to_run,
			)
			.context("failed to write code block")?;

			main_script.push(format!(
				"\
					import {{{} as CodeBlock_{code_block_index}}} from './codeblock_{code_block_index}.js';\n\
					applications.push(bootstrapApplication(CodeBlock_{code_block_index}, {{providers: makeProviders(CodeBlock_{code_block_index})}}));\n\
				",
				&code_block.class_name
			));
		}

		let script_basename = project_folder.clone();

		let angular_main = format!("./{}/{}", &project_folder, &script_basename);
		self.write(
			root.join(format!("{angular_main}.ts")),
			&main_script.join("\n"),
		)
		.context("failed to write main chapter import")?;

		self.chapter_to_angular_file.push((
			chapter_path.to_string_lossy().into_owned(),
			format!("{angular_main}.js"),
		));

		Ok(())
	}

	pub(super) fn write_main<P: AsRef<Path>>(&self, config: &Config, root: P) -> Result<()> {
		let mut main_script =
			Vec::with_capacity(3 + config.polyfills.len() + self.chapter_to_angular_file.len());

		if !config.polyfills.contains(&"zone.js".to_owned()) {
			main_script.push("import 'zone.js';".to_owned());
		}

		for polyfill in &config.polyfills {
			main_script.push(format!("import '{polyfill}';"));
		}

		main_script.push("\nconst mods = new Map([".to_owned());

		for (chapter_path, relative_script_path) in &self.chapter_to_angular_file {
			main_script.push(format!(
				r#"  ["{chapter_path}", () => import("{relative_script_path}")],"#,
			));
		}

		main_script.push("]);\n\nmods.get((document.querySelector('#load-angular') as HTMLElement)?.dataset.path as string)?.();".to_owned());

		self.write_force(
			root.as_ref().join("load-angular.ts"),
			&main_script.join("\n"),
		)?;

		Ok(())
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
