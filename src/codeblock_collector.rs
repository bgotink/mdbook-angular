use anyhow::{Error, Result};
use once_cell::sync::Lazy;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};
use regex::Regex;

use crate::{codeblock::CodeBlock, utils::generated_rendered_code_block};

static COMMENT_WITHOUT_KEEP: Lazy<Regex> = Lazy::new(|| {
	Regex::new(r#"(\n?)\s*/\*\*(?s:@kee[^p]|@ke[^e]|@k[^e]|@[^k]|[^@])*\*/\s*?\n"#).unwrap()
});
static COMMENT_KEEP_START: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(/\*\*)\s*?@keep\b"#).unwrap());
static COMMENT_KEEP_MIDDLE: Lazy<Regex> =
	Lazy::new(|| Regex::new(r#"(\n)\s*(\*\s*)?@keep\s*?\n"#).unwrap());

struct CurrentCcodeBlock {
	language: Box<str>,
	content: Option<Box<str>>,
}

#[derive(Default)]
pub(crate) struct CodeBlockCollector {
	include_playgrounds: bool,
	has_playgrounds: bool,

	err: Option<Error>,
	code_blocks: Vec<CodeBlock>,
	current_code_block: Option<CurrentCcodeBlock>,
}

impl CodeBlockCollector {
	pub(crate) fn new(include_playgrounds: bool) -> Self {
		CodeBlockCollector {
			include_playgrounds,
			..Default::default()
		}
	}

	pub(crate) fn process_event<'a>(&mut self, event: Event<'a>) -> Vec<Event<'a>> {
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
				vec![Event::Text(text)]
			};
		}

		if let pulldown_cmark::Event::End(pulldown_cmark::Tag::CodeBlock(
			pulldown_cmark::CodeBlockKind::Fenced(lang),
		)) = &event
		{
			let Some(current_code_block) = &self.current_code_block else {
				return vec![event];
			};

			let Some(text) = &current_code_block.content else {
				return vec![event];
			};

			let add_playground = if lang.contains("no-playground") {
				false
			} else if lang.contains("playground") {
				true
			} else {
				self.include_playgrounds
			};

			let index = self.code_blocks.len();

			let code_block = match CodeBlock::new(text, index) {
				Ok(code_block) => code_block,
				Err(err) => {
					log::error!("Failed to parse angular code block\nDid you mean this to be an angular code sample?");

					if self.err.is_none() {
						self.err = Some(err);
					}

					// return value won't matter, finalize() will throw the error
					return vec![];
				}
			};

			let rendered_codeblock = generated_rendered_code_block(
				&code_block,
				index,
				add_playground,
				&mut self.has_playgrounds,
			);

			self.code_blocks.push(code_block);

			let mut events: Vec<Event<'a>> = Vec::new();

			if !lang.contains("hide") {
				events.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(
					CowStr::Boxed(current_code_block.language.clone()),
				))));

				let text = COMMENT_WITHOUT_KEEP.replace_all(text, "$1");
				let text = COMMENT_KEEP_START.replace_all(&text, "$1");
				let text = COMMENT_KEEP_MIDDLE.replace_all(&text, "$1");

				events.push(Event::Text(CowStr::Boxed(text.into())));

				events.push(Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(
					CowStr::Boxed(current_code_block.language.clone()),
				))));
			}

			events.push(Event::Html(rendered_codeblock.into()));

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
