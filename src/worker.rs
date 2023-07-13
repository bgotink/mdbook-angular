extern crate alloc;

use anyhow::Result;
use mdbook::{book::Chapter, renderer::RenderContext};
use once_cell::sync::Lazy;
use pulldown_cmark::Parser;
use regex::Regex;
use serde_json::json;
use std::{
	collections::HashSet,
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

	include_playgrounds: bool,
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

		let optimize = ctx
			.config
			.get("output.angular.optimize")
			.and_then(toml::Value::as_bool)
			.unwrap_or(true);

		Ok(AngularWorker {
			book_root: ctx.source_dir(),
			angular_root: root,
			target: ctx.destination.clone(),
			workspace: AngularWorkspace::new(optimize),
			index: 0,
			include_playgrounds,
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

		generate_angular_code(&self.angular_root.join(&project_name), angular_code_blocks)?;

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

		for chapter_path in self
			.chapters_with_playgrounds
			.iter()
			.chain(vec![Path::new("index.html").to_path_buf()].iter())
		{
			self.insert_angular_scripts_into_chapter(chapter_path)?;
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

	fn insert_angular_scripts_into_chapter(&self, chapter_path: &Path) -> Result<()> {
		static LOAD_ANGULAR_RE: Lazy<Regex> = Lazy::new(|| {
			Regex::new(r#"(?i)<script\s*load-angular-from="([^"]+)">\s*</script>"#).unwrap()
		});
		static SCRIPT_RE: Lazy<Regex> =
			Lazy::new(|| Regex::new(r"<script[^>]*></script>").unwrap());

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

		let Some(captures) = LOAD_ANGULAR_RE.captures(chapter.as_str()) else { return Ok(())};

		let project_name = captures.get(1).unwrap().as_str();
		let script_folder = self.target.join(project_name);

		let index: String = fs::read(script_folder.join("index.html"))?
			.into_iter()
			.map(|b| -> char { b.into() })
			.collect();

		let scripts = SCRIPT_RE
			.find_iter(&index)
			.map(|m| {
				m.as_str().replace(
					r#"src=""#,
					format!(r#"src="{}/{}/"#, path_to_root(&chapter_path), project_name).as_str(),
				)
			})
			.collect::<String>();

		chapter_file.rewind()?;
		chapter_file.write_all(
			chapter
				.replace(captures.get(0).unwrap().as_str(), scripts.as_str())
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
