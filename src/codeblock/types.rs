use std::rc::Rc;

use super::playground::Playground;

pub(crate) struct PrintedCodeBlock {
	/// The code to show in a markdown code block
	pub(crate) code: Rc<String>,
	/// Whether to show the code block collapsed or not
	pub(crate) collapsed: bool,
}

/// A block of angular code that will be shown with a live code sample
pub struct CodeBlock {
	/// The code to show on the page
	pub(crate) code_to_print: Option<PrintedCodeBlock>,

	/// An entire TypeScript file to write to disk
	pub(crate) code_to_run: Rc<String>,
	/// Name of the angular component exported in `code_to_run` that should be
	/// bootstrapped
	pub(crate) class_name: String,

	/// Whether to insert the element angular will bootstrap into the page
	pub(crate) insert: bool,
	/// The tag name of the root element
	pub(crate) tag: String,

	/// Playground for the live angular component, if enabled and present
	pub(crate) playground: Option<Playground>,
}
