extern crate alloc;

use mdbook::{book::Chapter, errors::Error, renderer::RenderContext};
use once_cell::sync::Lazy;
use pulldown_cmark::{CowStr, Parser};
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
	borrow::Borrow,
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
	process,
};

use crate::codeblock::CodeBlock;

const TAG_ANGULAR: &str = "angular";

static CODEBLOCK_IO_SCRIPT: &[u8] = include_bytes!("codeblock-io.js");

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
	has_playgrounds: bool,
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
			has_playgrounds: false,
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

		let events = Parser::new(&chapter.content).flat_map(|e| {
			if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(
				pulldown_cmark::CodeBlockKind::Fenced(lang),
			)) = &e
			{
				current_angular_code_block = if lang.contains(TAG_ANGULAR) {
					Some(String::new())
				} else {
					None
				};
				return vec![e];
			}

			if let pulldown_cmark::Event::Text(text) = e {
				if let Some(current_angular_code_block) = current_angular_code_block.as_mut() {
					current_angular_code_block.push_str(&text.to_string());

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

					match CodeBlock::new(&angular_code, index) {
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
												.as_ref()
												.map(|s| s.as_str())
												.unwrap_or(""),
											index,
											serde_json::to_string(&input.config).unwrap().replace("<", "&lt;")
										)
									})
									.collect::<String>();

								element = format!(
									"{}\n{}",
									element,
									format!(
										"\
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
										inputs,
									)
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

			return vec![e];
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
					"optimization": false /*{
						"scripts": true,
						"styles": {
							"minify": true,
							"inlineCritical": false
						},
						"fonts": false
					}*/,
					// "outputHashing": "all",
					"polyfills": ["zone.js"],
					"progress": false,
					"tsConfig": format!("code_{}/tsconfig.json", index)
				}),
			},
		);

		new_content.push_str(
			format!("\n\n<script>fetch(`${{path_to_root || '.'}}/{0}/include.html`).then(r => r.text()).then(t => Array.from(new DOMParser().parseFromString(t,'text/html').querySelectorAll('script')).forEach((s,c)=>{{c=document.createElement(s.tagName);Array.from(s.attributes).forEach(a=>c.setAttribute(a.name,a.name==='src'?`${{path_to_root || '.'}}/{0}/${{a.value}}`:a.value));document.body.appendChild(c)}}))</script>\n", &project_name).as_str()
		);

		if has_playgrounds {
			self.has_playgrounds = true;

			new_content.push_str(
				"<script type=\"module\">import(`${path_to_root || '.'}/codeblock-io.js`)</script>\n"
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
		fs::write(
			Path::join(&self.root, "angular.json"),
			serde_json::to_string(&self.workspace)?,
		)?;

		let script_re = Regex::new(r"<script[^>]*></script>")?;

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

			let index: String = fs::read(Path::join(&script_folder, "index.html"))?
				.into_iter()
				.map(|b| -> char { b.into() })
				.collect();

			let scripts: String = script_re.find_iter(&index).map(|m| m.as_str()).collect();

			fs::write(Path::join(&script_folder, "include.html"), scripts)?;
		}

		if self.has_playgrounds {
			fs::write(
				Path::join(&self.target, "codeblock-io.js"),
				CODEBLOCK_IO_SCRIPT,
			)?;
		}

		Ok(())
	}
}
