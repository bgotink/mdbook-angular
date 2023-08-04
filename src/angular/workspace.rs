use std::{collections::HashMap, fs, path::Path};

use anyhow::Context;
use serde::Serialize;
use serde_json::Value;

use crate::Result;

#[derive(Debug, Clone, PartialEq, Serialize)]
struct AngularWorkspaceCliSettings {
	analytics: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct AngularWorkspace {
	version: i32,
	cli: AngularWorkspaceCliSettings,
	projects: HashMap<String, AngularWorkspaceProject>,
}

impl AngularWorkspace {
	pub(super) fn new() -> Self {
		AngularWorkspace {
			version: 1,
			cli: AngularWorkspaceCliSettings { analytics: false },
			projects: HashMap::new(),
		}
	}

	pub(super) fn add_project(&mut self, name: &str, root: &str) -> &mut AngularWorkspaceProject {
		self.projects.insert(
			name.to_owned(),
			AngularWorkspaceProject {
				root: root.to_owned(),
				project_type: "application".to_owned(),
				architect: HashMap::new(),
			},
		);

		self.projects.get_mut(name).unwrap()
	}

	pub(super) fn write(&self, root: &Path) -> Result<()> {
		fs::write(
			root.join("angular.json"),
			serde_json::to_string(self).context("Failed to generate angular.json")?,
		)
		.context("Failed to write angular.json")?;
		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct AngularWorkspaceProject {
	root: String,
	#[serde(rename = "projectType")]
	project_type: String,
	architect: HashMap<String, AngularWorkspaceTarget>,
}

impl AngularWorkspaceProject {
	pub(super) fn add_target(&mut self, target: &str, builder: &str, options: Value) {
		self.architect.insert(
			target.to_owned(),
			AngularWorkspaceTarget {
				builder: builder.to_owned(),
				options,
			},
		);
	}
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct AngularWorkspaceTarget {
	builder: String,
	options: Value,
}
