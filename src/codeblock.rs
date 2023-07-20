use std::{io, path::Path, rc::Rc};

use anyhow::{Error, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use swc_common::{
	comments::{Comments, SingleThreadedComments},
	errors::{Handler, HANDLER},
	source_map::Pos,
	BytePos, FileName, SourceFile, Span, Spanned,
};
use swc_ecmascript::{
	ast,
	ast::EsVersion,
	parser,
	parser::{Syntax, TsConfig},
	visit::VisitWith,
};

static TS_EXT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\.([cm]?)ts(x?)$").unwrap());
static START_OF_FILE: BytePos = BytePos(1);

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PlaygroundInputType {
	#[default]
	Text,
	Number,
	Boolean,
	Enum(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub(crate) struct PlaygroundInputConfig {
	#[serde(rename = "type", default)]
	pub(crate) type_: PlaygroundInputType,

	#[serde(rename = "default")]
	pub(crate) default_value: Option<Value>,
}

#[derive(Debug)]
pub(crate) struct PlaygroundInput {
	pub(crate) name: String,
	pub(crate) description: Option<String>,
	pub(crate) config: PlaygroundInputConfig,
}

#[derive(Debug)]
pub(crate) struct PlaygroundAction {
	pub(crate) name: String,
	pub(crate) description: String,
}

struct CodeBlockVisitor<'a> {
	index: usize,
	source: &'a str,
	source_file: SourceFile,
	comments: SingleThreadedComments,

	tag: Option<String>,
	class_name: Option<String>,
	inputs: Vec<PlaygroundInput>,
	actions: Vec<PlaygroundAction>,

	overwritten_source: Option<String>,
}

impl<'a> CodeBlockVisitor<'a> {
	fn extract_selector(&mut self, component: &ast::ObjectLit, insert_if_needed: bool) {
		static INDENTATION: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^\s+"#).unwrap());

		let tag = component
			.props
			.iter()
			.filter_map(ast::PropOrSpread::as_prop)
			.filter_map(|p| p.as_key_value())
			.find(|p| match &p.key {
				ast::PropName::Ident(ident) => ident.sym.eq("selector"),
				ast::PropName::Str(str) => str.raw.as_ref().is_some_and(|v| v.eq("selector")),
				_ => false,
			});

		if let Some(selector) = tag {
			log::debug!(
				"Found a selector: {}-{}",
				selector.span_lo().0,
				selector.span_hi().0
			);
			if let Some(ast::Lit::Str(ast::Str { value, .. })) = selector.value.as_lit() {
				self.tag = Some(value.to_string());
			}

			return;
		}

		if !insert_if_needed {
			// Not filling out the tag, we'll throw on that later when trying to
			// extract the tag
			return;
		}

		self.tag = Some(format!("codeblock-{}", self.index));

		if let Some(first_prop) = component.props.first() {
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

			let insert = format!(
				"selector: '{}',\n{}",
				self.tag.as_ref().unwrap(),
				indentation
			);

			let (before, after) = self.source.split_at(span.lo.to_usize() - 1);
			self.overwritten_source = Some(format!("{before}{insert}{after}"));
		} else {
			log::warn!("Empty @Component() is not supported");
		}
	}

	fn visit_class_members(&mut self, class: &ast::Class) {
		for member in &class.body {
			log::debug!("class member");

			if let Some(decorators) = get_decorators(member) {
				log::debug!("found {} decorators", decorators.len());

				if includes_decorator_with_name(decorators, "Input") {
					self.extract_input(member);
				}
			};

			if let ast::ClassMember::Method(method) = member {
				self.extract_action(method);
			}
		}
	}

	fn extract_input(&mut self, member: &ast::ClassMember) {
		let Some(name) = prop_name(member) else {
			return;
		};

		let mut input = PlaygroundInput {
			name,
			description: None,
			config: PlaygroundInputConfig::default(),
		};

		if let Some(value) = prop_value(member) {
			try_extract_default(value, &mut input.config);
		}

		let leading_comment = self.get_leading_comment(member.span_lo());

		if let Some(comment) = leading_comment {
			let mut description: Vec<_> = Vec::new();

			for line in comment.lines() {
				if let Some(config) = line.trim_start().strip_prefix("@input") {
					match serde_json::from_str::<PlaygroundInputConfig>(config) {
						Ok(config) => {
							input.config.type_ = config.type_;
							if config.default_value.is_some() {
								input.config.default_value = config.default_value;
							}
						}
						Err(err) => {
							log::error!("Failed to parse input `{line}`: {err}");
						}
					};
					break;
				}

				description.push(line);
			}

			if !description.is_empty() {
				input.description = Some(description.join("\n"));
			}
		}

		self.inputs.push(input);
	}

	fn extract_action(&mut self, method: &ast::ClassMethod) {
		let Some(comment) = self.get_leading_comment(method.span_lo()) else { return };

		if comment.contains("@action") {
			let (ast::PropName::Ident(ast::Ident { sym: name, .. })
			| ast::PropName::Str(ast::Str { value: name, .. })) = &method.key else { return };
			let name = name.to_string();
			let description = comment.replace("@action", "").trim().to_owned();

			self.actions.push(PlaygroundAction { name, description });
		}
	}

	fn get_leading_comment(&self, span_lo: BytePos) -> Option<String> {
		self.comments
			.get_leading(span_lo)
			.and_then(|list| list.into_iter().next())
			.map(|comment| {
				static COMMENT_LINE_PREFIX: Lazy<Regex> =
					Lazy::new(|| Regex::new(r"(?m)^[ \t]*\*(?:[ \t]|$)").unwrap());

				let comment = comment.text.trim();
				let comment = comment.strip_prefix("/*").unwrap_or(comment);
				let comment = comment.strip_suffix("*/").unwrap_or(comment);

				let comment = COMMENT_LINE_PREFIX.replace_all(comment, "");

				comment.to_string()
			})
	}

	fn visit_exported_class(&mut self, name: String, n: &ast::Class, mut span: Span) {
		log::debug!(r#"visiting class "{name}""#);

		if let Some(name_to_look_for) = &self.class_name {
			if name_to_look_for.ne(&name) {
				return;
			}
		}

		for decorator in &n.decorators {
			span = span.to(decorator.span());

			if let Some(call) = decorator.expr.as_call() {
				if let Some(ident) = call.callee.as_expr().and_then(|c| c.as_ident()) {
					if !ident.sym.eq("Component") || call.args.len() != 1 {
						continue;
					}

					if let Some(obj) = call.args.first().unwrap().expr.as_object() {
						self.extract_selector(obj, self.class_name.is_none());

						break;
					}
				}
			}
		}

		if self.tag.is_none() {
			return;
		}

		if self.class_name.is_none() {
			self.class_name = Some(name);
		} else {
			if let Some(comments) = self.comments.take_leading(span.span_lo()) {
				for comment in comments {
					span = span.to(comment.span);
				}
			}

			self.source = &self.source
				[(span.lo - START_OF_FILE).to_usize()..(span.hi - START_OF_FILE).to_usize()];
		}

		self.visit_class_members(n);
	}
}

impl<'a> swc_ecmascript::visit::Visit for CodeBlockVisitor<'a> {
	fn visit_export_decl(&mut self, n: &ast::ExportDecl) {
		if self.tag.is_some() {
			return;
		}

		let span = n.span();
		let Some(n) = n.decl.as_class() else { return };
		let name = n.ident.sym.to_string();

		self.visit_exported_class(name, &n.class, span);
	}

	fn visit_export_default_decl(&mut self, n: &ast::ExportDefaultDecl) {
		if self.tag.is_some() {
			return;
		}

		let span = n.span();
		let Some(n) = n.decl.as_class() else { return };

		self.visit_exported_class("default".to_owned(), &n.class, span);
	}
}

fn prop_name(prop: &ast::ClassMember) -> Option<String> {
	let (ast::ClassMember::ClassProp(ast::ClassProp { key, .. })
		| ast::ClassMember::AutoAccessor(ast::AutoAccessor {
			key: ast::Key::Public(key),
			..
		})) = prop else { return None; };

	match key {
		ast::PropName::Ident(ident) => Some(ident.sym.to_string()),
		ast::PropName::Str(str) => Some(str.value.to_string()),
		_ => None,
	}
}

fn prop_value(prop: &ast::ClassMember) -> Option<&ast::Expr> {
	match prop {
		ast::ClassMember::ClassProp(ast::ClassProp { value, .. })
		| ast::ClassMember::AutoAccessor(ast::AutoAccessor { value, .. }) => value.as_deref(),
		_ => None,
	}
}

fn get_decorators(prop: &ast::ClassMember) -> Option<&Vec<ast::Decorator>> {
	match prop {
		ast::ClassMember::AutoAccessor(prop) => Some(&prop.decorators),
		ast::ClassMember::ClassProp(prop) => Some(&prop.decorators),
		_ => None,
	}
}

fn includes_decorator_with_name(decorators: &[ast::Decorator], name: &str) -> bool {
	decorators.iter().any(|decorator| {
		if let Some(call) = decorator.expr.as_call() {
			if let Some(ast::Expr::Ident(ident)) =
				call.callee.as_expr().map(std::convert::AsRef::as_ref)
			{
				ident.sym.eq(name)
			} else {
				false
			}
		} else {
			false
		}
	})
}

fn try_extract_default(expr: &ast::Expr, input: &mut PlaygroundInputConfig) {
	let Some(lit) = expr.as_lit() else { return };

	match lit {
		ast::Lit::Bool(value) => {
			input.type_ = PlaygroundInputType::Boolean;
			input.default_value = Some(Value::Bool(value.value));
		}
		ast::Lit::Num(value) => {
			input.type_ = PlaygroundInputType::Number;
			input.default_value = Some(json!(value.value));
		}
		ast::Lit::Str(value) => {
			input.type_ = PlaygroundInputType::Text;
			input.default_value = Some(Value::String(value.value.to_string()));
		}
		_ => {}
	}
}

pub(crate) struct CodeBlock {
	pub(crate) source_to_show: String,
	pub(crate) source_to_write: String,
	pub(crate) tag: String,
	pub(crate) class_name: String,
	pub(crate) inputs: Vec<PlaygroundInput>,
	pub(crate) actions: Vec<PlaygroundAction>,
}

impl CodeBlock {
	pub(crate) fn new(
		source: &str,
		index: usize,
		class_name: Option<&str>,
		reexport_path: Option<&Path>,
	) -> Result<CodeBlock> {
		let handler = Handler::with_emitter_writer(Box::new(io::stderr()), None);

		let source_file = SourceFile::new_from(
			FileName::Anon,
			false,
			FileName::Anon,
			Rc::new(source.to_owned()),
			START_OF_FILE,
		);

		let comments = SingleThreadedComments::default();

		let program = parser::parse_file_as_program(
			&source_file,
			Syntax::Typescript(TsConfig {
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

		let mut code_block = CodeBlockVisitor {
			index,
			source,
			source_file,
			tag: None,
			class_name: class_name.map(ToOwned::to_owned),
			comments,
			inputs: Vec::new(),
			actions: Vec::new(),
			overwritten_source: None,
		};

		HANDLER.set(&handler, || program.visit_with(&mut code_block));

		let Some(class_name) = code_block.class_name else {
			return Err(
				match class_name {
					Some(class_name) => Error::msg(format!("Failed to find class {class_name}")),
					None => Error::msg("Failed to find component class")
				}
			);
		};

		let Some(tag) = code_block.tag else {
			return Err(Error::msg(format!("Failed to find selector on class {class_name}")));
		};

		let source_to_show = code_block
			.overwritten_source
			.unwrap_or_else(|| code_block.source.to_owned());

		let source_to_write = match reexport_path {
			Some(reexport_path) => {
				// TypeScript/JavaScript only support string paths, so... this should be
				// fine otherwise things will not work, regardless of whether we can
				// successfully print the path into the file.
				let reexport_path = reexport_path.as_os_str().to_string_lossy();

				let reexport_path = TS_EXT.replace_all(reexport_path.as_ref(), "$1js$2");

				format!("export {{{class_name}}} from './{reexport_path}';\n")
			}
			None => source_to_show.clone(),
		};

		Ok(CodeBlock {
			source_to_show,
			source_to_write,
			tag,
			class_name,
			inputs: code_block.inputs,
			actions: code_block.actions,
		})
	}
}

#[cfg(test)]
mod test {
	use anyhow::Result;
	use serde_json::{from_str, to_string, Number, Value};

	use crate::codeblock::{PlaygroundInputConfig, PlaygroundInputType};

	#[test]
	fn option_json_format() -> Result<()> {
		assert_eq!(
			to_string(&PlaygroundInputConfig {
				default_value: Some(Value::String("Bram".to_owned())),
				type_: PlaygroundInputType::Text,
			})?,
			r#"{"type":"text","default":"Bram"}"#
		);

		assert_eq!(
			to_string(&PlaygroundInputConfig {
				default_value: Some(Value::Number(Number::from(42))),
				type_: PlaygroundInputType::Number,
			})?,
			r#"{"type":"number","default":42}"#
		);

		assert_eq!(
			to_string(&PlaygroundInputConfig {
				default_value: None,
				type_: PlaygroundInputType::Boolean,
			})?,
			r#"{"type":"boolean","default":null}"#
		);

		assert_eq!(
			to_string(&PlaygroundInputConfig {
				default_value: None,
				type_: PlaygroundInputType::Enum(vec!["one".to_owned(), "two".to_owned()]),
			})?,
			r#"{"type":{"enum":["one","two"]},"default":null}"#
		);

		Ok(())
	}

	#[test]
	fn option_empty_type() -> Result<()> {
		assert_eq!(
			from_str::<PlaygroundInputConfig>("{}")?,
			PlaygroundInputConfig {
				default_value: None,
				type_: PlaygroundInputType::Text,
			},
		);

		Ok(())
	}
}
