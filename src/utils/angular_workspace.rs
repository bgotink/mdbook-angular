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
}

impl AngularWorkspace {
	pub(crate) fn new(optimize: bool) -> Self {
		AngularWorkspace {
			optimize,
			data: AngularWorkspaceData {
				version: 1,
				projects: HashMap::new(),
			},
		}
	}

	pub(crate) fn add_project(&mut self, name: &str) {
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
					"inlineStyleLanguage": "scss", // TODO make configurable
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

		serde_json::to_writer(file, &self.data)?;

		Ok(AngularRunner { root, target })
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
}

impl<'a> AngularRunner<'a> {
	pub(crate) fn run(&self, project_name: &str) -> Result<()> {
		let script_folder = self.target.join(project_name);

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
