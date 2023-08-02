#[derive(PartialEq, Eq, Debug)]
pub(super) enum CodeBlockFlags {
	/// Do not show the source code
	Hide,
	/// Show the source code collapsed
	Collapsed,
	/// Show the source code uncollapsed
	Uncollapsed,

	/// Show a playground even if configuration disables them
	Playground,
	/// Do not show a playground even if configuration allows them
	NoPlayground,

	/// Do not insert the Angular root element into the page
	NoInsert,
}

fn to_flag(value: &str) -> Option<CodeBlockFlags> {
	match value {
		"hide" => Some(CodeBlockFlags::Hide),
		"playground" => Some(CodeBlockFlags::Playground),
		"noplayground" | "no-playground" => Some(CodeBlockFlags::NoPlayground),
		"uncollapsed" | "no-collapse" => Some(CodeBlockFlags::Uncollapsed),
		"collapsed" | "collapse" => Some(CodeBlockFlags::Collapsed),
		"no-insert" => Some(CodeBlockFlags::NoInsert),
		_ => None,
	}
}

fn is_flag_separator(c: char) -> bool {
	c == ',' || c == ' '
}

/// Extract flags from the given string
///
/// The text should contain flags separated by space or comma.
/// Unknown flags are ignored.
pub(super) fn get_flags(string: &str) -> Vec<CodeBlockFlags> {
	string
		.split(is_flag_separator)
		.filter_map(to_flag)
		.collect()
}
