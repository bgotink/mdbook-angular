use std::{io, rc::Rc};

use anyhow::{Error, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use swc_common::{
	comments::{Comments, SingleThreadedComments},
	errors::{Handler, HANDLER},
	source_map::Pos,
	FileName, SourceFile, Spanned,
};
use swc_ecmascript::{
	ast,
	ast::EsVersion,
	parser,
	parser::{Syntax, TsConfig},
	visit::VisitWith,
};

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
	pub(crate) _type: PlaygroundInputType,

	#[serde(rename = "default")]
	pub(crate) default_value: Option<Value>,
}

#[derive(Debug)]
pub(crate) struct PlaygroundInput {
	pub(crate) name: String,
	pub(crate) description: Option<String>,
	pub(crate) config: PlaygroundInputConfig,
}

struct CodeBlockVisitor {
	index: usize,
	source: Rc<String>,
	source_file: SourceFile,
	comments: SingleThreadedComments,

	tag: Option<String>,
	class_name: Option<String>,
	inputs: Vec<PlaygroundInput>,
}

impl CodeBlockVisitor {
	fn extract_selector(&mut self, component: &ast::ObjectLit) {
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
			self.source = Rc::new(vec![before, &insert, after].into_iter().collect());
		} else {
			log::warn!("Empty @Component() is not supported");
		}
	}

	fn extract_inputs(&mut self, class: &ast::Class) {
		for member in &class.body {
			log::debug!("class member");

			let Some(decorators) = get_decorators(member) else {
				continue;
			};

			log::debug!("found {} decorators", decorators.len());

			if includes_decorator_with_name(decorators, "Input") {
				self.extract_input(member);
			}
		}
	}

	fn extract_input(&mut self, member: &ast::ClassMember) {
		static COMMENT_LINE_PREAMBLE: Lazy<Regex> =
			Lazy::new(|| Regex::new(r#"^\s*\*\s*"#).unwrap());

		let Some(name) = prop_name(member) else {
			return;
		};

		let mut input = PlaygroundInput {
			name,
			description: None,
			config: PlaygroundInputConfig::default(),
		};

		let leading_comment = self
			.comments
			.get_leading(member.span_lo())
			.and_then(|list| list.into_iter().next());

		if let Some(comment) = leading_comment {
			let comment = comment.text.to_string();
			let mut description: Vec<_> = Vec::new();

			for line in comment.lines() {
				let clean_line = COMMENT_LINE_PREAMBLE.replace(line, "");

				if let Some(config) = clean_line.strip_prefix("@input") {
					match serde_json::from_str::<PlaygroundInputConfig>(config) {
						Ok(config) => input.config = config,
						Err(err) => {
							log::error!("Failed to parse input `{clean_line}`: {err}");
						}
					};
					break;
				}

				description.push(clean_line);
			}

			if !description.is_empty() {
				input.description = Some(description.join("\n"));
			}
		}

		self.inputs.push(input);
	}

	fn visit_exported_class(&mut self, n: &swc_ecmascript::ast::ClassDecl) {
		let name = n.ident.sym.to_string();
		log::debug!("visiting class {name}");
		let n = &n.class;

		for decorator in &n.decorators {
			if let Some(call) = decorator.expr.as_call() {
				if let Some(ident) = call.callee.as_expr().and_then(|c| c.as_ident()) {
					if !ident.sym.eq("Component") || call.args.len() != 1 {
						continue;
					}

					if let Some(obj) = call.args.first().unwrap().expr.as_object() {
						self.extract_selector(obj);

						break;
					}
				}
			}
		}

		if self.tag.is_none() {
			return;
		}

		self.class_name = Some(name);

		self.extract_inputs(n);
	}
}

impl swc_ecmascript::visit::Visit for CodeBlockVisitor {
	fn visit_export_decl(&mut self, n: &ast::ExportDecl) {
		if self.tag.is_some() || !n.decl.is_class() {
			return;
		}

		self.visit_exported_class(n.decl.as_class().unwrap());
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

pub(crate) struct CodeBlock {
	pub(crate) source: String,
	pub(crate) tag: String,
	pub(crate) class_name: String,
	pub(crate) inputs: Vec<PlaygroundInput>,
}

impl CodeBlock {
	pub(crate) fn new(source: &str, index: usize) -> Result<CodeBlock> {
		let handler = Handler::with_emitter_writer(Box::new(io::stderr()), None);
		let source = Rc::new(source.to_owned());

		let source_file = SourceFile::new_from(
			FileName::Anon,
			false,
			FileName::Anon,
			source.clone(),
			swc_common::BytePos(1),
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
			class_name: None,
			comments,
			inputs: Vec::new(),
		};

		HANDLER.set(&handler, || program.visit_with(&mut code_block));

		Ok(CodeBlock {
			source: code_block.source.to_string(),
			tag: code_block
				.tag
				.ok_or(Error::msg("Failed to find component selector"))?,
			class_name: code_block
				.class_name
				.ok_or(Error::msg("Failed to find component class name"))?,
			inputs: code_block.inputs,
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
				_type: PlaygroundInputType::Text,
			})?,
			r#"{"type":"text","default":"Bram"}"#
		);

		assert_eq!(
			to_string(&PlaygroundInputConfig {
				default_value: Some(Value::Number(Number::from(42))),
				_type: PlaygroundInputType::Number,
			})?,
			r#"{"type":"number","default":42}"#
		);

		assert_eq!(
			to_string(&PlaygroundInputConfig {
				default_value: None,
				_type: PlaygroundInputType::Boolean,
			})?,
			r#"{"type":"boolean","default":null}"#
		);

		assert_eq!(
			to_string(&PlaygroundInputConfig {
				default_value: None,
				_type: PlaygroundInputType::Enum(vec!["one".to_owned(), "two".to_owned()]),
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
				_type: PlaygroundInputType::Text,
			},
		);

		Ok(())
	}
}
