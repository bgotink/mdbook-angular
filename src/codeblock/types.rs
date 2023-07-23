use std::rc::Rc;

use super::playground::Playground;

pub(crate) struct CodeBlock {
	pub(crate) code_to_print: Rc<String>,
	pub(crate) code_to_run: Rc<String>,
	pub(crate) collapsed: bool,
	pub(crate) playground: Option<Playground>,
	pub(crate) tag: String,
	pub(crate) class_name: String,
	pub(crate) insert: bool,
}
