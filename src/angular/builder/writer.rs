use std::{fs, path::Path};

use serde_json::json;

use crate::{Config, Context, Result, markdown::CollectedCodeBlock};

pub(super) struct Writer<'a> {
	changed_only: bool,
	config: &'a Config,
	chapter_to_angular_file: Vec<(String, String)>,
}

impl<'a> Writer<'a> {
	pub(super) fn new(config: &'a Config, changed_only: bool) -> Self {
		Self {
			changed_only,
			config,
			chapter_to_angular_file: Vec::new(),
		}
	}
}

impl Writer<'_> {
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
		code_blocks: Vec<CollectedCodeBlock>,
	) -> Result<()> {
		let root = root.as_ref();
		let project_folder = format!("code_{index}");

		let absolute_project_folder = root.join(&project_folder);

		fs::create_dir_all(&absolute_project_folder).context("failed to create project folder")?;

		let mut main_script = Vec::with_capacity(1 + code_blocks.len());

		if self.config.zoneless {
			main_script.push(
          "\n\
              import {provideZonelessChangeDetection, type ApplicationRef, type Provider, type EnvironmentProviders, type Type} from '@angular/core';\n\
              import {bootstrapApplication} from '@angular/platform-browser';\n\
              function makeProviders(component: Type<unknown> & {rootProviders?: readonly (Provider | EnvironmentProviders)[] | null | undefined}) {\n\
                  return [provideZonelessChangeDetection(), ...(component.rootProviders ?? [])];\n\
              }\n\
              const applications: Promise<ApplicationRef>[] = [];\n\
              (globalThis as any).mdBookAngular = {zone: null, applications};\n\
          "
          .to_owned(),
      );
		} else {
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
		}

		for code_block in code_blocks {
			let code_block_index = code_block.index;
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
				code_block.class_name
			));
		}

		let script_basename = project_folder.clone();

		let angular_main = format!("./{project_folder}/{script_basename}");
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

	pub(super) fn write_main<P: AsRef<Path>>(&self, root: P) -> Result<()> {
		let mut main_script = Vec::with_capacity(
			3 + self.config.polyfills.len() + self.chapter_to_angular_file.len(),
		);

		for polyfill in &self.config.polyfills {
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

	pub(super) fn write_tsconfig(&self) -> Result<()> {
		let tsconfig = if let Some(tsconfig) = &self.config.tsconfig {
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
			self.config.angular_root_folder.join("tsconfig.json"),
			&serde_json::to_string(&tsconfig)?,
		)
		.context("failed to write tsconfig.json")?;

		Ok(())
	}
}
