#[derive(PartialEq, Eq, Debug)]
pub(super) enum CodeBlockFlags {
	Hide,
	Playground,
	NoPlayground,
	Collapsed,
}

fn to_flag(value: &str) -> Option<CodeBlockFlags> {
	match value {
		"hide" => Some(CodeBlockFlags::Hide),
		"playground" => Some(CodeBlockFlags::Playground),
		"noplayground" | "no-playground" => Some(CodeBlockFlags::NoPlayground),
		"collapsed" | "collapse" => Some(CodeBlockFlags::Collapsed),
		_ => None,
	}
}

fn is_flag_separator(c: char) -> bool {
	c == ',' || c == ' '
}

pub(super) fn get_flags(language: &str) -> Vec<CodeBlockFlags> {
	language
		.split(is_flag_separator)
		.filter_map(to_flag)
		.collect()
}
