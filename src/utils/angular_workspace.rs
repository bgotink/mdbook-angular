extern crate alloc;

use anyhow::{Error, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
	collections::{hash_map::Keys, HashMap},
	fs,
	path::Path,
	process,
};

static EXPERIMENTAL_BUILDER_SCRIPT: &[u8] = include_bytes!("experimental-builder/builder.mjs");
static EXPERIMENTAL_BUILDER_CONFIG: &[u8] = include_bytes!("experimental-builder/schema.json");
static EXPERIMENTAL_BUILDER_PKG: &[u8] = include_bytes!("experimental-builder/package.json");

#[derive(Debug, Clone, PartialEq, Serialize)]
struct AngularWorkspaceData {
	version: i32,
	projects: HashMap<String, AngularWorkspaceProject>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct AngularWorkspaceProject {
	root: String,
	#[serde(rename = "projectType")]
	project_type: String,
	architect: HashMap<String, AngularWorkspaceTarget>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct AngularWorkspaceTarget {
	builder: String,
	options: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AngularWorkspace {
	data: AngularWorkspaceData,
	optimize: bool,
	inline_style_language: String,
	experimental: bool,
}

impl AngularWorkspace {
	pub(crate) fn new(
		experimental: bool,
		optimize: bool,
		inline_style_language: Option<&str>,
	) -> Self {
		let inline_style_language = inline_style_language.unwrap_or("css");

		let data = if experimental {
			let mut architect = HashMap::new();

			architect.insert(
				"build".to_owned(),
				AngularWorkspaceTarget {
					builder: "experimental-builder:application".to_owned(),
					options: json!({
						"optimization": optimize,
						"inlineStyleLanguage": inline_style_language
					}),
				},
			);

			let mut projects = HashMap::new();

			projects.insert(
				"application".to_owned(),
				AngularWorkspaceProject {
					root: String::new(),
					project_type: "application".to_owned(),
					architect,
				},
			);

			AngularWorkspaceData {
				version: 1,
				projects,
			}
		} else {
			AngularWorkspaceData {
				version: 1,
				projects: HashMap::new(),
			}
		};

		AngularWorkspace {
			data,
			experimental,
			optimize,
			inline_style_language: inline_style_language.to_owned(),
		}
	}

	pub(crate) fn add_project(&mut self, name: &str) {
		if self.experimental {
			return;
		}

		let (optimization, output_hashing) = if self.optimize {
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

		let mut architect: HashMap<String, AngularWorkspaceTarget> = HashMap::new();
		architect.insert(
			"build".into(),
			AngularWorkspaceTarget {
				builder: "@angular-devkit/build-angular:browser-esbuild".into(),
				options: json!({
					"commonChunk": false,
					"index": format!("{name}/index.html"),
					"inlineStyleLanguage": self.inline_style_language,
					"main": format!("{name}/main.ts"),
					"optimization": optimization,
					"outputHashing": output_hashing,
					"progress": false,
					"tsConfig": format!("{name}/tsconfig.json")
				}),
			},
		);

		self.data.projects.insert(
			name.to_owned(),
			AngularWorkspaceProject {
				root: name.to_owned(),
				project_type: "application".into(),
				architect,
			},
		);
	}

	pub(crate) fn projects(&self) -> ProjectIter {
		ProjectIter {
			inner: self.data.projects.keys(),
		}
	}

	pub(crate) fn write<'a>(&self, root: &'a Path, target: &'a Path) -> Result<AngularRunner<'a>> {
		let file = fs::OpenOptions::new()
			.write(true)
			.create(true)
			.open(root.join("angular.json"))?;

		if self.experimental {
			let experimental_builder_folder = root.join("node_modules/experimental-builder");
			fs::create_dir_all(&experimental_builder_folder)?;
			fs::write(
				experimental_builder_folder.join("package.json"),
				EXPERIMENTAL_BUILDER_PKG,
			)?;
			fs::write(
				experimental_builder_folder.join("builder.mjs"),
				EXPERIMENTAL_BUILDER_SCRIPT,
			)?;
			fs::write(
				experimental_builder_folder.join("schema.json"),
				EXPERIMENTAL_BUILDER_CONFIG,
			)?;
		}

		serde_json::to_writer(file, &self.data)?;

		Ok(AngularRunner {
			root,
			target,
			include_project_name_in_path: !self.experimental,
		})
	}
}

pub(crate) struct ProjectIter<'a> {
	inner: Keys<'a, String, AngularWorkspaceProject>,
}

impl<'a> Iterator for ProjectIter<'a> {
	type Item = &'a String;

	#[inline]
	fn next(&mut self) -> Option<&'a String> {
		self.inner.next()
	}

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}
}

pub(crate) struct AngularRunner<'a> {
	root: &'a Path,
	target: &'a Path,
	include_project_name_in_path: bool,
}

impl<'a> AngularRunner<'a> {
	pub(crate) fn run(&self, project_name: &str) -> Result<()> {
		let script_folder = if self.include_project_name_in_path {
			self.target.join(project_name)
		} else {
			self.target.to_owned()
		};

		let relative_script_folder = pathdiff::diff_paths(script_folder, self.root)
			.ok_or(Error::msg("Failed to compute relative output path"))?;
		let relative_script_folder_str = relative_script_folder
			.to_str()
			.ok_or(Error::msg("Failed to represent output path as string"))?;

		if !process::Command::new("yarn")
			.args([
				"ng",
				"build",
				project_name,
				"--output-path",
				relative_script_folder_str,
			])
			.current_dir(self.root)
			.stdout(process::Stdio::inherit())
			.status()?
			.success()
		{
			return Err(Error::msg(format!(
				"Failed to build project {project_name}",
			)));
		}

		Ok(())
	}
}
