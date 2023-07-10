extern crate alloc;

use mdbook::{book::Chapter, errors::Error, renderer::RenderContext};
use once_cell::sync::Lazy;
use pulldown_cmark::{CowStr, Parser};
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
	borrow::Borrow,
	collections::{HashMap, HashSet},
	fs,
	io::{Read, Write},
	path::{Path, PathBuf},
	process,
};

use crate::codeblock::CodeBlock;

const TAG_ANGULAR: &str = "angular";

static CODEBLOCK_IO_SCRIPT: &[u8] = include_bytes!("codeblock-io.min.js");

#[derive(Debug, Clone, PartialEq, Serialize)]
struct AngularWorkspace {
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

pub(crate) struct AngularWorker {
	root: PathBuf,
	target: PathBuf,
	workspace: AngularWorkspace,
	index: u32,

	include_playgrounds: bool,
	chapters_with_playgrounds: HashSet<PathBuf>,

	optimize: bool,
}

impl AngularWorker {
	pub(crate) fn new(ctx: &RenderContext) -> Result<AngularWorker, Error> {
		let mut root = ctx.root.clone();

		if let Some(toml::Value::String(angular_root)) = ctx.config.get("output.angular.root") {
			root.push(angular_root);
		}

		root.push(".angular/mdbook");

		if root.try_exists()? {
			fs::remove_dir_all(&root)?;
		}

		fs::create_dir_all(&root)?;

		if let Some(toml::Value::String(tsconfig)) = ctx.config.get("output.angular.tsconfig") {
			let resolved_tsconfig = Path::join(&root, tsconfig);

			fs::write(
				Path::join(&root, "tsconfig.json"),
				serde_json::to_string(&json!({ "extends": resolved_tsconfig }))?,
			)?;
		} else {
			fs::write(
                Path::join(&root, "tsconfig.json"),
                "{\"compilerOptions\":{\"strict\": true,\"sourceMap\": true,\"experimentalDecorators\": true,\"moduleResolution\": \"node\",\"importHelpers\": true,\"target\": \"ES2022\",\"module\": \"ES2022\",\"useDefineForClassFields\": false,\"lib\": [\"ES2022\",\"dom\"]}}"
            )?;
		}

		let include_playgrounds = ctx
			.config
			.get("output.angular.playgrounds")
			.and_then(|v| v.as_bool())
			.unwrap_or(true);

		let optimize = ctx
			.config
			.get("output.angular.optimize")
			.and_then(|v| v.as_bool())
			.unwrap_or(true);

		Ok(AngularWorker {
			// switch to std::path::absolute once stable
			root: root.canonicalize()?,
			target: ctx.destination.clone(),
			workspace: AngularWorkspace {
				version: 1,
				projects: HashMap::new(),
			},
			index: 0,
			include_playgrounds,
			chapters_with_playgrounds: HashSet::new(),
			optimize,
		})
	}

	pub(crate) fn process_chapter(&mut self, chapter: &mut Chapter) -> Result<(), Error> {
		static COMMENT_WITHOUT_KEEP: Lazy<Regex> = Lazy::new(|| {
			Regex::new(r#"(\n?)\s*/\*\*(?s:@kee[^p]|@ke[^e]|@k[^e]|@[^k]|[^@])*\*/\s*?\n"#).unwrap()
		});
		static COMMENT_KEEP_START: Lazy<Regex> =
			Lazy::new(|| Regex::new(r#"(/\*\*)\s*?@keep\b"#).unwrap());
		static COMMENT_KEEP_MIDDLE: Lazy<Regex> =
			Lazy::new(|| Regex::new(r#"(\n)\s*(\*\s*)?@keep\s*?\n"#).unwrap());

		let mut angular_code_samples: Vec<CodeBlock> = Vec::new();

		let mut current_angular_code_block: Option<String> = None;
		let mut error: Option<Error> = None;
		let mut has_playgrounds = false;

		let (can_have_playgrouds, path_to_root) = if let Some(path) = &chapter.path {
			(true, path_to_root(path))
		} else {
			(false, "".into())
		};

		let events = Parser::new(&chapter.content).flat_map(|e| {
			if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(
				pulldown_cmark::CodeBlockKind::Fenced(lang),
			)) = &e
			{
				current_angular_code_block = if lang.contains(TAG_ANGULAR) {
					if can_have_playgrouds {
						Some(String::new())
					} else {
						log::error!("Cannot add playgrounds in chapter {} as it has no path", &chapter.name);
						None
					}
				} else {
					None
				};
				return vec![e];
			}

			if let pulldown_cmark::Event::Text(text) = e {
				if let Some(current_angular_code_block) = current_angular_code_block.as_mut() {
					current_angular_code_block.push_str(&text);

					let text = COMMENT_WITHOUT_KEEP.replace_all(text.borrow(), "$1");
					let text = COMMENT_KEEP_START.replace_all(&text, "$1");
					let text = COMMENT_KEEP_MIDDLE.replace_all(&text, "$1");

					return vec![pulldown_cmark::Event::Text(CowStr::from(text.to_string()))];
				} else {
					return vec![pulldown_cmark::Event::Text(text)];
				}
			}

			if let pulldown_cmark::Event::End(pulldown_cmark::Tag::CodeBlock(
				pulldown_cmark::CodeBlockKind::Fenced(lang),
			)) = &e
			{
				if let Some(angular_code) = &current_angular_code_block {
					let playground = if lang.contains("no-playground") {
						false
					} else if lang.contains("playground") {
						true
					} else {
						self.include_playgrounds
					};

					let index = angular_code_samples.len();

					match CodeBlock::new(angular_code, index) {
						Ok(sample) => {
							let mut element = format!("<{0}></{0}>\n", &sample.tag);

							if playground && !sample.inputs.is_empty() {
								has_playgrounds = true;

								let inputs = sample
									.inputs
									.iter()
									.map(|input| {
										format!(
											"<tr><td><code class=\"hljs\">{}</code></td><td>{}</td><td><mdbook-angular-input name=\"{0}\" index=\"{}\">{}</mdbook-angular-input></td></tr>",
											&input.name,
											input
												.description
												.as_deref()
												.unwrap_or(""),
											index,
											serde_json::to_string(&input.config).unwrap().replace('<', "&lt;")
										)
									})
									.collect::<String>();

								element = format!(
									"\
										{}\n\
										Inputs:\n\n\
										<table>\
											<thead>\
												<th>Name</th>
												<th>Description</th>
												<th>Value</th>
											</thead>\
											<tbody>{}</tbody>\
										</table>\n\n\
									",
									element,
									inputs,
								);
							}

							angular_code_samples.push(sample);

							return vec![e, pulldown_cmark::Event::Html(CowStr::from(element))];
						}
						Err(err) => {
							log::error!("Failed to parse angular code block\nDid you mean this to be an angular code sample?");

							if error.is_none() {
								error = Some(err);
							}

							// return value doesn't matter, we'll return an error below anyway
							return vec![e];
						}
					}
				}
				current_angular_code_block = None;
			}

			vec![e]
		});

		let mut new_content: String = String::with_capacity(chapter.content.len());
		pulldown_cmark_to_cmark::cmark(events, &mut new_content)?;

		if let Some(err) = error {
			return Err(err);
		}

		if angular_code_samples.is_empty() {
			return Ok(());
		}

		let index = self.index;
		self.index += 1;

		let project_name = format!("code_{}", index);
		let project_root = Path::join(&self.root, &project_name);
		fs::create_dir(&project_root)?;

		let mut main = String::new();
		main.push_str("import 'zone.js';\n");
		main.push_str("import {NgZone, ApplicationRef} from '@angular/core';\n");
		main.push_str("import {bootstrapApplication} from '@angular/platform-browser';\n");
		main.push_str("const zone = new NgZone({});\n");
		main.push_str("const providers = [{provide: NgZone, useValue: zone}];\n");
		main.push_str("const applications: Promise<ApplicationRef>[] = [];\n");
		main.push_str("(globalThis as any).mdBookAngular = {zone, applications};");

		for (index, file) in angular_code_samples.into_iter().enumerate() {
			fs::write(
				Path::join(&project_root, format!("component_{}.ts", index + 1)),
				file.source,
			)?;

			main.push_str(
        format!(
					"\nimport {{{1} as CodeBlock_{0}}} from './component_{0}';\napplications.push(bootstrapApplication(CodeBlock_{0}, {{providers}}));\n",
					index + 1,
					file.class_name,
				).as_str()
			);
		}

		fs::write(Path::join(&project_root, "main.ts"), main)?;

		fs::write(
			Path::join(&project_root, "index.html"),
			"<!doctype html>\n<html></html>\n",
		)?;
		fs::write(
			Path::join(&project_root, "tsconfig.json"),
			"{\"extends\":\"../tsconfig.json\",\"files\": [\"main.ts\"]}",
		)?;

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
					"index": format!("code_{}/index.html", index),
					"inlineStyleLanguage": "scss", // TODO make configurable
					"main": format!("code_{}/main.ts", index),
					"optimization": optimization,
					"outputHashing": output_hashing,
					"progress": false,
					"tsConfig": format!("code_{}/tsconfig.json", index)
				}),
			},
		);

		new_content.push_str(
			format!(
				"\n\n<script load-angular-from=\"{}\"></script>\n",
				&project_name
			)
			.as_str(),
		);

		if has_playgrounds {
			self.chapters_with_playgrounds
				.insert(chapter.path.as_ref().unwrap().clone());

			new_content.push_str(
				format!(
					"<script type=\"module\" src=\"{}/codeblock-io.js\"></script>\n",
					path_to_root
				)
				.as_str(),
			);
		}

		chapter.content = new_content;

		self.workspace.projects.insert(
			project_name.clone(),
			AngularWorkspaceProject {
				root: project_name,
				project_type: "application".into(),
				architect,
			},
		);

		Ok(())
	}

	pub(crate) fn finalize(&self) -> Result<(), Error> {
		static LOAD_ANGULAR_RE: Lazy<Regex> = Lazy::new(|| {
			Regex::new(r#"(?i)<script\s*load-angular-from="([^"]+)">\s*</script>"#).unwrap()
		});
		static SCRIPT_RE: Lazy<Regex> =
			Lazy::new(|| Regex::new(r"<script[^>]*></script>").unwrap());

		fs::write(
			Path::join(&self.root, "angular.json"),
			serde_json::to_string(&self.workspace)?,
		)?;

		for project_name in self.workspace.projects.keys() {
			let script_folder = Path::join(&self.target, project_name);

			let relative_script_folder = pathdiff::diff_paths(&script_folder, &self.root)
				.ok_or(Error::msg("Failed to compute relative output path"))?;
			let relative_script_folder_str = relative_script_folder
				.to_str()
				.ok_or(Error::msg("Failed to represent output path as string"))?;

			if !process::Command::new("yarn")
				.args([
					"ng",
					"build",
					project_name.as_str(),
					"--output-path",
					relative_script_folder_str,
				])
				.current_dir(&self.root)
				.stdout(process::Stdio::inherit())
				.status()?
				.success()
			{
				return Err(Error::msg(format!(
					"Failed to build project {}",
					project_name
				)));
			}
		}

		for chapter_path in self
			.chapters_with_playgrounds
			.iter()
			.chain(vec![Path::new("index.html").to_path_buf()].iter())
		{
			let mut chapter_path = chapter_path.clone();
			if !chapter_path.set_extension("html") {
				continue;
			}

			let mut chapter_file = fs::OpenOptions::new()
				.read(true)
				.write(true)
				.create(false)
				.open(self.target.join(&chapter_path))?;
			let mut chapter = String::new();
			chapter_file.read_to_string(&mut chapter)?;

			if let Some(captures) = LOAD_ANGULAR_RE.captures(chapter.as_str()) {
				let project_name = captures.get(1).unwrap().as_str();
				let script_folder = Path::join(&self.target, project_name);

				let index: String = fs::read(Path::join(&script_folder, "index.html"))?
					.into_iter()
					.map(|b| -> char { b.into() })
					.collect();

				let scripts = SCRIPT_RE
					.find_iter(&index)
					.map(|m| {
						m.as_str().replace(
							r#"src=""#,
							format!(r#"src="{}/{}/"#, path_to_root(&chapter_path), project_name)
								.as_str(),
						)
					})
					.collect::<String>();

				chapter_file.write_all(
					chapter
						.replace(captures.get(0).unwrap().as_str(), scripts.as_str())
						.as_bytes(),
				)?;
			};
		}

		if !self.chapters_with_playgrounds.is_empty() {
			fs::write(
				Path::join(&self.target, "codeblock-io.js"),
				CODEBLOCK_IO_SCRIPT,
			)?;
		}

		Ok(())
	}
}

fn path_to_root(path: &Path) -> String {
	let mut parts = Vec::new();
	let mut current = path.parent().unwrap();

	while let Some(parent) = current.parent() {
		if current == parent {
			break;
		}

		parts.push("..");
		current = parent;
	}

	if parts.is_empty() {
		".".into()
	} else {
		parts.join("/")
	}
}
