use std::{fs, path::Path};

use anyhow::{Error, Result};
use once_cell::sync::Lazy;
use pathdiff::diff_paths;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};
use regex::Regex;

use crate::{codeblock::CodeBlock, utils::generated_rendered_code_block};

static COMMENT_WITHOUT_KEEP: Lazy<Regex> = Lazy::new(|| {
	Regex::new(r#"(\n?)[ \t]*/\*\*(?s:@kee[^p]|@ke[^e]|@k[^e]|@[^k]|\*[^/]|[^*@])*\*/[ \t]*?\n"#)
		.unwrap()
});
static COMMENT_KEEP_START: Lazy<Regex> =
	Lazy::new(|| Regex::new(r#"(/\*\*)[ \t]*?@keep\b"#).unwrap());
static COMMENT_KEEP_MIDDLE: Lazy<Regex> =
	Lazy::new(|| Regex::new(r#"(\n)[ \t]*(\*[ \t]*)?@keep[ \t]*?\n"#).unwrap());

static TAG_ANGULAR: Lazy<Regex> = Lazy::new(|| {
	Regex::new(
		r#"\{\{#angular\s+(?<path>\S+?)(?:#(?<class_name>\S+))?(?<flags>(?:\s+(?:hide|no-playground|playground)?)*)\}\}"#,
	)
	.unwrap()
});

struct CurrentCcodeBlock {
	language: Box<str>,
	content: Option<Box<str>>,
}

pub(crate) struct CodeBlockCollector<'a> {
	include_playgrounds: bool,
	has_playgrounds: bool,

	book_root: &'a Path,
	angular_root: &'a Path,
	chapter_path: &'a Path,

	err: Option<Error>,
	code_blocks: Vec<CodeBlock>,
	current_code_block: Option<CurrentCcodeBlock>,
}

impl<'a> CodeBlockCollector<'a> {
	pub(crate) fn new(
		book_root: &'a Path,
		angular_root: &'a Path,
		chapter_path: &'a Path,
		include_playgrounds: bool,
	) -> Self {
		CodeBlockCollector {
			include_playgrounds,
			has_playgrounds: false,
			book_root,
			angular_root,
			chapter_path,

			err: None,
			code_blocks: Vec::new(),
			current_code_block: None,
		}
	}

	fn store_err(&mut self, err: Error) {
		if self.err.is_none() {
			self.err = Some(err);
		}
	}

	fn insert_code_block(
		&mut self,
		source: &str,
		lang: &str,
		events: &mut Vec<Event>,
		class_name: Option<&str>,
		reexport_path: Option<&Path>,
	) {
		let add_playground = if lang.contains("no-playground") {
			false
		} else if lang.contains("playground") {
			true
		} else {
			self.include_playgrounds
		};

		let index = self.code_blocks.len();

		let code_block = match CodeBlock::new(source, index, class_name, reexport_path) {
			Ok(code_block) => code_block,
			Err(err) => {
				log::error!("Failed to parse angular code block\nDid you mean this to be an angular code sample?");
				self.store_err(err);

				// return value won't matter, finalize() will throw the error
				return;
			}
		};

		let rendered_codeblock = generated_rendered_code_block(
			&code_block,
			index,
			add_playground,
			&mut self.has_playgrounds,
		);

		if !lang.contains("hide") {
			events.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(
				CowStr::Boxed(lang.to_owned().into_boxed_str()),
			))));

			let text = COMMENT_WITHOUT_KEEP.replace_all(&code_block.source_to_show, "$1");
			let text = COMMENT_KEEP_START.replace_all(&text, "$1");
			let text = COMMENT_KEEP_MIDDLE.replace_all(&text, "$1");

			events.push(Event::Text(CowStr::Boxed(text.into())));

			events.push(Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(
				CowStr::Boxed(lang.to_owned().into_boxed_str()),
			))));
		}

		self.code_blocks.push(code_block);

		events.push(Event::Html(rendered_codeblock.into()));
	}

	pub(crate) fn process_event<'i>(&mut self, event: Event<'i>) -> Vec<Event<'i>> {
		if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) = event {
			return if lang.contains("angular") {
				self.current_code_block = Some(CurrentCcodeBlock {
					language: lang.to_string().into_boxed_str(),
					content: None,
				});
				vec![]
			} else {
				self.current_code_block = None;

				vec![Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))]
			};
		}

		if let Event::Text(text) = event {
			return if let Some(current_code_block) = &mut self.current_code_block {
				current_code_block.content = Some(text.into_string().into_boxed_str());
				vec![]
			} else {
				let mut text = text.as_ref();
				let mut events = Vec::new();

				while let Some(c) = TAG_ANGULAR.captures(text) {
					let m = c.get(0).unwrap();

					events.push(Event::Text(CowStr::Boxed(
						text[..m.start()].to_owned().into_boxed_str(),
					)));

					let path = self.book_root.join(self.chapter_path.parent().unwrap());
					let path = path.join(&c["path"]);

					let contents = match fs::read_to_string(&path) {
						Ok(content) => content,
						Err(err) => {
							self.store_err(Error::new(err).context(format!(
								"Failed to read angular playground file at {} in {}",
								&c["path"],
								self.chapter_path.display()
							)));

							// we'll throw the error later anyway
							return vec![];
						}
					};

					let mut flags = vec!["ts", "angular"];
					flags.append(&mut c["flags"].split_whitespace().collect::<Vec<&str>>());

					let reexport_path =
						diff_paths(&path, self.angular_root.join("does_not_matter"));

					self.insert_code_block(
						&contents,
						&flags.join(","),
						&mut events,
						c.name("class_name").map(|m| m.as_str()),
						reexport_path.as_deref(),
					);

					let end = m.end();
					if end < text.len() {
						text = &text[(m.end() + 1)..];
					} else {
						text = "";
					}
				}

				if !text.is_empty() {
					events.push(Event::Text(CowStr::Boxed(text.to_owned().into_boxed_str())));
				}

				events
			};
		}

		if let pulldown_cmark::Event::End(pulldown_cmark::Tag::CodeBlock(
			pulldown_cmark::CodeBlockKind::Fenced(_),
		)) = &event
		{
			let Some(current_code_block) = self.current_code_block.take() else {
				return vec![event];
			};

			let Some(text) = current_code_block.content else {
				return vec![event];
			};

			let mut events: Vec<Event<'i>> = Vec::new();
			self.insert_code_block(&text, &current_code_block.language, &mut events, None, None);
			return events;
		}

		vec![event]
	}

	pub(crate) fn finalize(self) -> Result<(bool, Vec<CodeBlock>)> {
		match self.err {
			Some(err) => Err(err),
			None => Ok((self.has_playgrounds, self.code_blocks)),
		}
	}
}
