mod flags;
mod parser;
pub(crate) mod playground;
mod types;

use std::path::Path;

use anyhow::Result;
pub(crate) use types::{CodeBlock, PrintedCodeBlock};

use crate::config::Config;

use self::{
	flags::get_flags,
	parser::{parse_codeblock, ParsedCodeBlock},
};

pub(crate) fn is_angular_codeblock(language: &str) -> bool {
	language.contains("angular")
}

pub(crate) fn to_codeblock<L: AsRef<str>, C: AsRef<str>>(
	config: &Config,
	index: usize,
	class_name: Option<&str>,
	reexport_path: Option<&Path>,
	language: L,
	code: C,
	code_to_print: &Option<C>,
) -> Result<CodeBlock> {
	let language = language.as_ref().to_owned();
	let code = code.as_ref();

	let flags = get_flags(&language);

	let hidden = flags.contains(&flags::CodeBlockFlags::Hide);
	let collapsed = flags.contains(&flags::CodeBlockFlags::Collapsed);

	let insert = !flags.contains(&flags::CodeBlockFlags::NoInsert);

	let allow_playground = if flags.contains(&flags::CodeBlockFlags::NoPlayground) {
		false
	} else if flags.contains(&flags::CodeBlockFlags::Playground) {
		true
	} else {
		config.playgrounds
	};

	let ParsedCodeBlock {
		code_to_print,
		code_to_run,
		playground,
		class_name,
		tag,
	} = parse_codeblock(
		code,
		code_to_print.as_ref().map(AsRef::as_ref),
		allow_playground,
		index,
		class_name,
		reexport_path,
	)?;

	let code_to_print = if hidden {
		None
	} else {
		Some(PrintedCodeBlock {
			code: code_to_print,
			collapsed,
		})
	};

	Ok(CodeBlock {
		code_to_print,
		code_to_run,
		class_name,
		insert,
		tag,
		playground,
	})
}
