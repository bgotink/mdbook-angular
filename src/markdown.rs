extern crate alloc;

use std::{
	fs,
	path::{Path, PathBuf},
	rc::Rc,
};

use mdbook::book::Chapter;
use once_cell::sync::Lazy;
use pathdiff::diff_paths;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Parser, Tag};
use pulldown_cmark_to_cmark::cmark as markdown_to_string;
use regex::Regex;

use crate::{
	codeblock::{is_angular_codeblock, to_codeblock, CodeBlock, PrintedCodeBlock},
	Config, Error, Result,
};

struct CodeBlockCollector<'a> {
	config: &'a Config,
	chapter: &'a Chapter,

	code_blocks: Vec<CodeBlock>,

	in_codeblock: bool,
	current_code: Option<String>,

	error: Result<()>,
}

impl<'a> CodeBlockCollector<'a> {
	fn new(config: &'a Config, chapter: &'a Chapter) -> Self {
		CodeBlockCollector {
			config,
			chapter,
			code_blocks: Vec::new(),

			in_codeblock: false,
			current_code: None,

			error: Ok(()),
		}
	}

	fn process_event<'b>(&mut self, event: Event<'b>) -> ProcessedEvent<'b> {
		static TAG_ANGULAR: Lazy<Regex> = Lazy::new(|| {
			Regex::new(
				r#"\{\{#angular\s+(?<path>\S+?)(?:#(?<class_name>\S+))?(?<flags>(?:\s+(?:hide|no-playground|playground|collapsed)?)*)\}\}"#,
			)
			.unwrap()
		});

		if self.error.is_err() {
			return ProcessedEvent::empty();
		}

		if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language))) = &event {
			if is_angular_codeblock(language) {
				self.in_codeblock = true;
				return ProcessedEvent::empty();
			}
		}

		if self.in_codeblock {
			if let Event::Text(text) = &event {
				self.current_code = Some(text.to_string());
				return ProcessedEvent::empty();
			}

			if let Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(language))) = &event {
				self.in_codeblock = false;

				let Some(code) = self.current_code.take() else {
					return ProcessedEvent::multiple(vec![
						Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language.clone()))),
						Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(language.clone()))),
					]);
				};

				return self.insert_code_block(None, None, language, &code, &Some(&code));
			}
		}

		let Event::Text(text) = &event else {
			return ProcessedEvent::single(event);
		};
		let mut text = text.as_ref();

		if !text.contains("#angular") {
			return ProcessedEvent::single(event);
		}

		let Some(chapter_path) = &self.chapter.path else {
			return ProcessedEvent::single(event);
		};

		let mut events = ProcessedEvent::empty();

		while let Some(captures) = TAG_ANGULAR.captures(text) {
			let match_ = captures.get(0).unwrap();

			events = events.concat(ProcessedEvent::single(Event::Text(CowStr::Boxed(
				text[..match_.start()].to_owned().into_boxed_str(),
			))));

			let path = self
				.config
				.book_source_folder
				.join(chapter_path.parent().unwrap());
			let path = path.join(&captures["path"]);

			let contents = match fs::read_to_string(&path) {
				Ok(content) => content,
				Err(err) => {
					self.error(Error::new(err).context(format!(
						"Failed to read angular playground file at {} in {}",
						&captures["path"],
						chapter_path.display()
					)));

					return ProcessedEvent::empty();
				}
			};

			let mut flags = vec!["ts", "angular"];
			flags.append(&mut captures["flags"].split_whitespace().collect::<Vec<&str>>());

			let reexport_path = diff_paths(
				&path,
				self.config.angular_root_folder.join("does_not_matter"),
			);

			events = events.concat(self.insert_code_block(
				captures.name("class_name").map(|m| m.as_str()),
				reexport_path.as_deref(),
				&flags.join(","),
				&contents,
				&None,
			));

			let end = match_.end();
			if end < text.len() {
				text = &text[(match_.end() + 1)..];
			} else {
				text = "";
			}
		}

		events
	}

	fn insert_code_block<'b, L: AsRef<str>, C: AsRef<str>>(
		&mut self,
		class_name: Option<&str>,
		reexport_path: Option<&Path>,
		language: L,
		code: C,
		code_to_print: &Option<C>,
	) -> ProcessedEvent<'b> {
		let index = self.code_blocks.len();
		let language = language.as_ref();

		match to_codeblock(
			self.config,
			index,
			class_name,
			reexport_path,
			language,
			code,
			code_to_print,
		) {
			Ok(code_block) => {
				let mut events = Vec::new();

				if let Some(PrintedCodeBlock { code, collapsed }) = &code_block.code_to_print {
					if *collapsed {
						events.push(Event::Html(CowStr::Boxed(
							"<details><summary>Show code</summary>\n\n"
								.to_owned()
								.into_boxed_str(),
						)));
					}

					events.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(
						CowStr::Boxed(language.to_owned().into_boxed_str()),
					))));

					events.push(Event::Text(CowStr::Boxed(
						Rc::as_ref(code).clone().into_boxed_str(),
					)));

					events.push(Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(
						CowStr::Boxed(language.to_owned().into_boxed_str()),
					))));

					if *collapsed {
						events.push(Event::Html(CowStr::Boxed(
							"</details>\n\n".to_owned().into_boxed_str(),
						)));
					}
				}

				events.push(Event::Html(CowStr::Boxed(
					print_playground(index, &code_block).into_boxed_str(),
				)));

				self.code_blocks.push(code_block);

				ProcessedEvent::multiple(events)
			}
			Err(error) => {
				self.error(error);
				ProcessedEvent::empty()
			}
		}
	}

	fn error(&mut self, error: Error) {
		if self.error.is_ok() {
			self.error = Err(error);
		}
	}
}

fn print_playground(index: usize, code_block: &CodeBlock) -> String {
	let mut result = Vec::new();

	if code_block.insert {
		result.push(format!("<{0}></{0}>\n", code_block.tag));
	}

	if let Some(playground) = &code_block.playground {
		if !playground.inputs.is_empty() {
			let inputs = playground
				.inputs
				.iter()
				.map(|input| {
					format!(
						"\
							<tr>\
								<td>\
									<code>{}</code>\
								</td>\
								<td>{}</td>\
								<td>\
									<mdbook-angular-input name=\"{0}\" index=\"{}\">{}</mdbook-angular-input>\
								</td>\
							</tr>\
						",
						&input.name,
						input.description.as_deref().unwrap_or(""),
						index,
						serde_json::to_string(&input.config)
							.unwrap()
							.replace('<', "&lt;")
					)
				})
				.collect::<String>();

			result.push(format!(
				"\n\
					Inputs:\n\n\
					<table class=\"mdbook-angular mdbook-angular-inputs\">\
						<thead>\
							<th>Name</th>
							<th>Description</th>
							<th>Value</th>
						</thead>\
						<tbody>{inputs}</tbody>\
					</table>\n\n\
				"
			));
		}

		if !playground.actions.is_empty() {
			let actions = playground
				.actions
				.iter()
				.map(|action| {
					format!(
						"\
							<tr>\
								<td>\
									<mdbook-angular-action name=\"{}\" index=\"{}\"></mdbook-angular-action>\
								</td>\
								<td>{}</td>\
							</tr>\
						",
						&action.name, index, action.description,
					)
				})
				.collect::<String>();

			result.push(format!(
				"\n\
					Actions:\n\n\
					<table class=\"mdbook-angular mdbook-angular-actions\">\
						<thead>\
							<th>Action</th>
							<th>Description</th>
						</thead>\
						<tbody>{actions}</tbody>\
					</table>\n\n\
				"
			));
		}
	}

	result.join("")
}

enum ProcessedEvent<'a> {
	Single(Option<Event<'a>>),
	Multiple(alloc::vec::IntoIter<Event<'a>>),
	Chain(Box<core::iter::Chain<ProcessedEvent<'a>, ProcessedEvent<'a>>>),
}

impl<'a> ProcessedEvent<'a> {
	fn empty() -> Self {
		Self::Single(None)
	}

	fn single(event: Event<'a>) -> Self {
		Self::Single(Some(event))
	}

	fn multiple(events: Vec<Event<'a>>) -> Self {
		Self::Multiple(events.into_iter())
	}

	fn concat(self, other: Self) -> Self {
		Self::Chain(Box::new(self.into_iter().chain(other.into_iter())))
	}
}

impl<'a> Iterator for ProcessedEvent<'a> {
	type Item = Event<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::Single(result) => result.take(),
			Self::Multiple(iter) => iter.next(),
			Self::Chain(iter) => iter.next(),
		}
	}
}

pub(crate) struct ChapterWithCodeBlocks {
	source_path: PathBuf,
	code_blocks: Vec<CodeBlock>,
	script_marker: String,
}

impl ChapterWithCodeBlocks {
	pub(crate) fn has_playgrounds(&self) -> bool {
		self.code_blocks
			.iter()
			.any(|block| block.playground.is_some())
	}

	pub(crate) fn script_marker(&self) -> &str {
		&self.script_marker
	}

	pub(crate) fn source_path(&self) -> &Path {
		&self.source_path
	}

	pub(crate) fn number_of_code_blocks(&self) -> usize {
		self.code_blocks.len()
	}
}

impl IntoIterator for ChapterWithCodeBlocks {
	type Item = CodeBlock;
	type IntoIter = std::vec::IntoIter<CodeBlock>;
	fn into_iter(self) -> Self::IntoIter {
		self.code_blocks.into_iter()
	}
}

pub(crate) fn process_markdown(
	config: &Config,
	chapter: &mut Chapter,
) -> Result<Option<ChapterWithCodeBlocks>> {
	let Some(source_path) = chapter.source_path.as_ref().map(Clone::clone) else {
		return Ok(None)
	};

	let mut new_content: String = String::with_capacity(chapter.content.len());
	let mut collector = CodeBlockCollector::new(config, chapter);

	markdown_to_string(
		Parser::new(&chapter.content).flat_map(|event| collector.process_event(event)),
		&mut new_content,
	)?;

	collector.error?;

	let code_blocks = collector.code_blocks;

	if code_blocks.is_empty() {
		return Ok(None);
	}

	let script_marker = format!(
		r#"load-angular-for="{}""#,
		source_path.as_os_str().to_string_lossy()
	);

	new_content.push_str(&format!(
		r#"{}<script type="module" {script_marker}></script>"#,
		"\n\n"
	));

	chapter.content = new_content;

	Ok(Some(ChapterWithCodeBlocks {
		source_path,
		code_blocks,
		script_marker,
	}))
}
