extern crate alloc;

use anyhow::Result;
use mdbook::{book::Chapter, renderer::RenderContext};
use once_cell::sync::Lazy;
use pulldown_cmark::Parser;
use regex::Regex;
use serde_json::json;
use std::{
	collections::{HashMap, HashSet},
	fs,
	io::{Read, Seek, Write},
	path::{Path, PathBuf},
};

use crate::{
	codeblock_collector::CodeBlockCollector,
	utils::{generate_angular_code, path_to_root, AngularWorkspace},
};

static CODEBLOCK_IO_SCRIPT: &[u8] = include_bytes!("codeblock-io.min.js");

pub(crate) struct AngularWorker {
	book_root: PathBuf,
	angular_root: PathBuf,
	target: PathBuf,
	workspace: AngularWorkspace,
	index: u32,

	experimental_builder: bool,
	include_playgrounds: bool,
	chapters_with_angular: HashSet<PathBuf>,
	chapters_with_playgrounds: HashSet<PathBuf>,
}

impl AngularWorker {
	pub(crate) fn new(ctx: &RenderContext) -> Result<AngularWorker> {
		let root = ctx.root.join("mdbook_angular");

		if root.try_exists()? {
			fs::remove_dir_all(&root)?;
		}

		fs::create_dir_all(&root)?;

		let root = root.canonicalize()?;
		log::debug!("Angular root folder {}", root.display());

		if let Some(toml::Value::String(tsconfig)) = ctx.config.get("output.angular.tsconfig") {
			let resolved_tsconfig = ctx.root.join(tsconfig);

			fs::write(
				root.join("tsconfig.json"),
				serde_json::to_string(&json!({
					"extends": resolved_tsconfig,
				}))?,
			)?;
		} else {
			fs::write(
				root.join("tsconfig.json"),
				serde_json::to_string(&json!({
					"compilerOptions":{
						"strict": true,
						"sourceMap": true,
						"experimentalDecorators": true,
						"moduleResolution": "node",
						"importHelpers": true,
						"target": "ES2022",
						"module": "ES2022",
						"useDefineForClassFields": false,
						"lib": ["ES2022", "dom"],
					}
				}))?,
			)?;
		}

		let include_playgrounds = ctx
			.config
			.get("output.angular.playgrounds")
			.and_then(toml::Value::as_bool)
			.unwrap_or(true);

		let experimental_builder = ctx
			.config
			.get("output.angular.experimentalBuilder")
			.and_then(toml::Value::as_bool)
			.unwrap_or(false);

		let optimize = ctx
			.config
			.get("output.angular.optimize")
			.and_then(toml::Value::as_bool)
			.unwrap_or(true);

		let inline_style_language = ctx
			.config
			.get("output.angular.inlineStyleLanguage")
			.and_then(toml::Value::as_str);

		Ok(AngularWorker {
			book_root: ctx.source_dir(),
			angular_root: root,
			target: ctx.destination.clone(),
			experimental_builder,
			workspace: AngularWorkspace::new(experimental_builder, optimize, inline_style_language),
			index: 0,
			include_playgrounds,
			chapters_with_angular: HashSet::new(),
			chapters_with_playgrounds: HashSet::new(),
		})
	}

	pub(crate) fn process_chapter(&mut self, chapter: &mut Chapter) -> Result<()> {
		let Some(chapter_path) = &chapter.path else { return Ok(()) };
		let path_to_root = path_to_root(chapter_path);

		let mut collector = CodeBlockCollector::new(
			&self.book_root,
			&self.angular_root,
			chapter_path,
			self.include_playgrounds,
		);

		let events = Parser::new(&chapter.content).flat_map(|e| collector.process_event(e));

		let mut new_content: String = String::with_capacity(chapter.content.len());
		pulldown_cmark_to_cmark::cmark(events, &mut new_content)?;

		let (has_playgrounds, angular_code_blocks) = collector.finalize()?;

		if angular_code_blocks.is_empty() {
			return Ok(());
		}

		let index = self.index;
		self.index += 1;

		let project_name = format!("code_{index}");

		generate_angular_code(
			&self.angular_root.join(&project_name),
			angular_code_blocks,
			self.experimental_builder,
		)?;

		self.chapters_with_angular.insert(chapter_path.clone());
		new_content.push_str(&format!(
			"\n\n<script load-angular-from=\"{project_name}\"></script>\n",
		));

		if has_playgrounds {
			self.chapters_with_playgrounds.insert(chapter_path.clone());

			new_content.push_str(&format!(
				"<script type=\"module\" src=\"{path_to_root}/codeblock-io.js\"></script>\n"
			));
		}

		chapter.content = new_content;

		self.workspace.add_project(&project_name);

		Ok(())
	}

	pub(crate) fn finalize(&self) -> Result<()> {
		self.build_angular_code()?;

		let scripts: HashMap<_, _> = if self.experimental_builder {
			fs::read_dir(&self.target)?
				.filter_map(std::result::Result::ok)
				.filter(is_js)
				.filter_map(to_string_name)
				.filter(|name| name.starts_with("code_"))
				.map(|name| {
					let dot = name.find('.').unwrap();
					(name[0..dot].to_owned(), name)
				})
				.collect()
		} else {
			self.workspace
				.projects()
				.filter_map(|name| {
					fs::read_dir(self.target.join(name))
						.ok()
						.and_then(|entries| {
							entries
								.filter_map(std::result::Result::ok)
								.filter(is_js)
								.filter_map(to_string_name)
								.find(|name| name.starts_with("main."))
								.map(|file| (name.clone(), format!("{name}/{file}")))
						})
				})
				.collect()
		};

		for chapter_path in self
			.chapters_with_angular
			.iter()
			.chain(std::iter::once(&Path::new("index.html").to_path_buf()))
		{
			self.insert_angular_scripts_into_chapter(chapter_path, &scripts)?;
		}

		self.write_playground_script()?;

		Ok(())
	}

	fn build_angular_code(&self) -> Result<()> {
		let runner = self.workspace.write(&self.angular_root, &self.target)?;

		for project_name in self.workspace.projects() {
			runner.run(project_name)?;
		}

		Ok(())
	}

	fn insert_angular_scripts_into_chapter(
		&self,
		chapter_path: &Path,
		scripts: &HashMap<String, String>,
	) -> Result<()> {
		static LOAD_ANGULAR_RE: Lazy<Regex> = Lazy::new(|| {
			Regex::new(r#"(?i)<script\s*load-angular-from="([^"]+)">\s*</script>"#).unwrap()
		});

		let mut chapter_path = chapter_path.to_path_buf();
		if !chapter_path.set_extension("html") {
			return Ok(());
		}

		let mut chapter_file = fs::OpenOptions::new()
			.read(true)
			.write(true)
			.create(false)
			.open(self.target.join(&chapter_path))?;
		let mut chapter = String::new();
		chapter_file.read_to_string(&mut chapter)?;

		let Some(captures) = LOAD_ANGULAR_RE.captures(&chapter) else { return Ok(())};

		let project_name = captures.get(1).unwrap().as_str();
		let Some(script) = scripts.get(project_name) else { return Ok(()) };

		let script = format!(
			r#"<script type="module" src="{}/{}"></script>"#,
			path_to_root(&chapter_path),
			script
		);

		chapter_file.rewind()?;
		chapter_file.write_all(
			chapter
				.replace(captures.get(0).unwrap().as_str(), &script)
				.as_bytes(),
		)?;

		Ok(())
	}

	fn write_playground_script(&self) -> Result<()> {
		if !self.chapters_with_playgrounds.is_empty() {
			fs::write(self.target.join("codeblock-io.js"), CODEBLOCK_IO_SCRIPT)?;
		}

		Ok(())
	}
}

fn is_js(entry: &fs::DirEntry) -> bool {
	entry
		.path()
		.extension()
		.map_or(false, |ext| ext.eq_ignore_ascii_case("js"))
}

#[allow(clippy::needless_pass_by_value)]
fn to_string_name(entry: fs::DirEntry) -> Option<String> {
	entry
		.file_name()
		.to_str()
		.map(std::borrow::ToOwned::to_owned)
}
