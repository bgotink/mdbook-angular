use std::{io, rc::Rc};

use anyhow::{Error, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use swc_common::{
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

pub(crate) struct PlaygroundInput {
	pub(crate) name: String,
	pub(crate) default_value: Option<String>,
	pub(crate) description: Option<String>,
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
	fn visit_exported_class(&mut self, n: &swc_ecmascript::ast::ClassDecl) {
		let name = n.ident.sym.to_string();
		log::debug!("visiting class {}", &name);
		let n = &n.class;
		static INDENTATION: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^\s+"#).unwrap());

		for decorator in n.decorators.iter() {
			if let Some(call) = decorator.expr.as_call() {
				if let Some(ident) = call.callee.as_expr().and_then(|c| c.as_ident()) {
					if !ident.sym.eq("Component") || call.args.len() != 1 {
						continue;
					}

					if let Some(obj) = call.args.first().unwrap().expr.as_object() {
						let tag = obj
							.props
							.iter()
							.filter(|p| p.is_prop())
							.map(|p| p.as_prop().unwrap())
							.filter(|p| p.is_key_value())
							.map(|p| p.as_key_value().unwrap())
							.find(|p| match &p.key {
								ast::PropName::Ident(ident) => ident.sym.eq("selector"),
								ast::PropName::Str(str) => {
									str.raw.as_ref().is_some_and(|v| v.eq("selector"))
								}
								_ => false,
							});

						if let Some(selector) = tag {
							log::debug!(
								"Found a selector: {}-{}",
								selector.span_lo().0,
								selector.span_hi().0
							);
							if let Some(ast::Lit::Str(ast::Str { value, .. })) =
								selector.value.as_lit()
							{
								self.tag = Some(value.to_string());
							}

							break;
						}

						self.tag = Some(format!("codeblock-{}", self.index));

						if let Some(first_prop) = obj.props.first() {
							let span = first_prop.span();

							let indentation = match self.source_file.lookup_line(span.lo) {
								Some(line) => {
									let line = self.source_file.get_line(line).unwrap();
									match INDENTATION.find(&line) {
										Some(m) => m.as_str().to_string(),
										_ => "".to_string(),
									}
								}
								_ => "  ".to_string(),
							};

							let insert = format!(
								"selector: '{}',\n{}",
								self.tag.as_ref().unwrap(),
								indentation
							);

							let (before, after) = self.source.split_at(span.lo.to_usize() - 1);
							self.source =
								Rc::new(vec![before, &insert, after].into_iter().collect());
						} else {
							log::warn!("Empty @Component() is not supported");
						}

						break;
					}
				}
			}
		}

		if self.tag == None {
			return;
		}

		self.class_name = Some(name);
	}
}

impl swc_ecmascript::visit::Visit for CodeBlockVisitor {
	fn visit_export_decl(&mut self, n: &ast::ExportDecl) {
		if self.tag != None || !n.decl.is_class() {
			return;
		}

		self.visit_exported_class(n.decl.as_class().unwrap());
	}
}

pub(crate) struct CodeBlock {
	pub(crate) source: String,
	pub(crate) tag: String,
	pub(crate) class_name: String,
}

impl CodeBlock {
	pub(crate) fn new(source: &String, index: usize) -> Result<CodeBlock> {
		let handler = Handler::with_emitter_writer(Box::new(io::stderr()), None);
		let source = Rc::new(source.clone());

		let source_file = SourceFile::new_from(
			FileName::Anon,
			false,
			FileName::Anon,
			source.clone(),
			swc_common::BytePos(1),
		);

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
			None,
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
		})
	}
}
