use std::{io, path::Path, rc::Rc};

use log::debug;
use once_cell::sync::Lazy;
use regex::Regex;
use swc_core::{
	common::{
		comments::SingleThreadedComments,
		errors::{Handler, HANDLER},
		source_map::SmallPos,
		BytePos, FileName, SourceFile, Span, Spanned,
	},
	ecma::{
		ast::{self, EsVersion},
		parser::{self, Syntax, TsSyntax},
	},
};

use crate::{utils::swc::get_decorator, Error, Result};

use super::playground::{parse_playground, Playground};

static TS_EXT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\.([cm]?)ts(x?)$").unwrap());
static START_OF_FILE: BytePos = BytePos(1);

pub(super) struct ParsedCodeBlock {
	pub(super) code_to_run: Rc<String>,
	pub(super) code_to_print: Rc<String>,
	pub(super) playground: Option<Playground>,
	pub(super) class_name: String,
	pub(super) tag: String,
}

struct CodeBlockVisitor {
	index: Option<usize>,
	source: Rc<String>,
	source_file: SourceFile,
	comments: SingleThreadedComments,
	code_to_print: Option<String>,
	allow_playground: bool,
	playground: Option<Playground>,
	tag: Option<String>,
	class_name: Option<String>,
}

impl CodeBlockVisitor {
	fn get_selector(&mut self, decorator: &ast::ObjectLit, name: &str) -> Result<String> {
		static INDENTATION: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s+").unwrap());

		let selector = decorator
			.props
			.iter()
			.filter_map(ast::PropOrSpread::as_prop)
			.map(Box::as_ref)
			.filter_map(ast::Prop::as_key_value)
			.find(|kv| match &kv.key {
				ast::PropName::Ident(ident) => ident.sym.eq("selector"),
				ast::PropName::Str(str) => str.value.eq("selector"),
				_ => false,
			});

		if let Some(selector) = selector {
			let selector = selector.value.as_lit().and_then(|lit| match lit {
				ast::Lit::Str(selector) => Some(selector.value.as_ref()),
				_ => None,
			});

			return if let Some(selector) = selector {
				Ok(selector.to_owned())
			} else {
				Err(Error::msg(format!(
					"Selector isn't a string literal in class {name}"
				)))
			};
		}

		let Some(generated_selector) = self.index.map(|i| format!("codeblock-{i}")) else {
			return Err(Error::msg(format!(
				"Coudldn't find selector on class {name}"
			)));
		};

		let Some(first_prop) = decorator.props.first() else {
			return Err(Error::msg(format!(
				"Unexpected empty @Component annotation in {name}"
			)));
		};

		let span = first_prop.span();

		let indentation = match self.source_file.lookup_line(span.lo) {
			Some(line) => {
				let line = self.source_file.get_line(line).unwrap();
				match INDENTATION.find(&line) {
					Some(m) => m.as_str().to_owned(),
					_ => String::new(),
				}
			}
			_ => "  ".to_owned(),
		};

		let insert = format!("selector: '{generated_selector}',\n{indentation}");

		let (before, after) = self
			.source
			.split_at(span.lo.to_usize() - START_OF_FILE.to_usize());

		let mut overwritten_source =
			String::with_capacity(before.len() + insert.len() + after.len());

		overwritten_source.push_str(before);
		overwritten_source.push_str(&insert);
		overwritten_source.push_str(after);

		self.code_to_print = Some(overwritten_source);

		Ok(generated_selector)
	}

	fn visit_exported_class(&mut self, name: &str, node: &ast::Class) -> Result<()> {
		if let Some(expected_name) = &self.class_name {
			if name.ne(expected_name) {
				return Ok(());
			}
		}

		debug!("Visiting class {name}");

		let Some(component) = get_decorator(&node.decorators, "Component") else {
			return Ok(());
		};

		debug!("found @Component on {name}");

		let Some(component) = component
			.expr
			.as_call()
			.and_then(|call| call.args.first())
			.and_then(|arg| arg.expr.as_object())
		else {
			return Ok(());
		};

		if self.tag.is_some() {
			return Err(Error::msg(format!(
				"File contains more than one exported component class: {} and {}",
				self.tag.as_ref().unwrap(),
				name
			)));
		}

		let selector = self.get_selector(component, name)?;

		self.tag = Some(selector);
		self.class_name = Some(name.to_owned());

		if self.allow_playground {
			self.playground = parse_playground(node, &self.comments)?;
		}

		if self.code_to_print.is_none() {
			let Span { hi, mut lo, .. } = node.span();

			for decorator in &node.decorators {
				let decorator_lo = decorator.span_lo();

				if decorator_lo < lo {
					lo = decorator_lo;
				}
			}

			self.code_to_print = Some(
				self.source[(lo - START_OF_FILE).to_usize()..(hi - START_OF_FILE).to_usize()]
					.to_owned(),
			);
		}

		Ok(())
	}

	fn visit_export_decl(&mut self, n: &ast::ExportDecl) -> Result<()> {
		if let Some(n) = n.decl.as_class() {
			self.visit_exported_class(&n.ident.sym, &n.class)?;
		}

		Ok(())
	}

	fn visit_export_default_decl(&mut self, n: &ast::ExportDefaultDecl) -> Result<()> {
		if let Some(n) = n.decl.as_class() {
			self.visit_exported_class("default", &n.class)?;
		};

		Ok(())
	}

	fn visit_program(&mut self, n: &ast::Program) -> Result<()> {
		let ast::Program::Module(module) = n else {
			return Err(Error::msg("Expected a module but got a script"));
		};

		for statement in &module.body {
			let ast::ModuleItem::ModuleDecl(decl) = statement else {
				continue;
			};

			match decl {
				ast::ModuleDecl::ExportDefaultDecl(n) => self.visit_export_default_decl(n)?,
				ast::ModuleDecl::ExportDecl(n) => self.visit_export_decl(n)?,
				_ => {}
			};
		}

		Ok(())
	}
}

pub(super) fn parse_codeblock(
	code: &str,
	code_to_print: Option<&str>,
	allow_playground: bool,
	index: usize,
	class_name: Option<&str>,
	reexport_path: Option<&Path>,
) -> Result<ParsedCodeBlock> {
	let code = Rc::new(code.to_owned());

	let handler = Handler::with_emitter_writer(Box::new(io::stderr()), None);

	let name: Rc<_> = FileName::Anon.into();

	let source_file = SourceFile::new_from(name.clone(), false, name, code.clone(), START_OF_FILE);

	let comments = SingleThreadedComments::default();

	let program = parser::parse_file_as_program(
		&source_file,
		Syntax::Typescript(TsSyntax {
			tsx: false,
			decorators: true,
			dts: false,
			no_early_errors: false,
			disallow_ambiguous_jsx_like: false,
		}),
		EsVersion::latest(),
		Some(&comments),
		&mut Vec::new(),
	)
	.map_err(|e| {
		e.into_diagnostic(&handler).emit();
		Error::msg("Failed to parse code block")
	})?;

	let mut visitor = CodeBlockVisitor {
		class_name: class_name.map(ToOwned::to_owned),
		source: code,
		source_file,
		comments,
		index: match reexport_path {
			Some(_) => None,
			None => Some(index),
		},
		allow_playground,
		playground: None,
		code_to_print: code_to_print.map(ToOwned::to_owned),
		tag: None,
	};

	HANDLER.set(&handler, || visitor.visit_program(&program))?;

	let Some(class_name) = visitor.class_name else {
		return Err(match class_name {
			Some(class_name) => Error::msg(format!("Failed to find class {class_name}")),
			None => Error::msg("Failed to find component class"),
		});
	};

	let Some(tag) = visitor.tag else {
		return Err(Error::msg(format!(
			"Failed to find selector on class {class_name}"
		)));
	};

	let code_to_print = visitor
		.code_to_print
		.map_or_else(|| visitor.source.clone(), Rc::new);

	let code_to_run = match reexport_path {
		Some(reexport_path) => {
			// TypeScript/JavaScript only support string paths, so... this should be
			// fine otherwise things will not work, regardless of whether we can
			// successfully print the path into the file.
			let reexport_path = reexport_path.as_os_str().to_string_lossy();

			let reexport_path = TS_EXT.replace_all(reexport_path.as_ref(), "$1js$2");

			Rc::new(format!(
				"export {{{class_name}}} from './{reexport_path}';\n"
			))
		}
		None => code_to_print.clone(),
	};

	let playground = visitor.playground;

	Ok(ParsedCodeBlock {
		code_to_run,
		code_to_print,
		playground,
		class_name,
		tag,
	})
}
