use anyhow::Result;
use serde_json::{Number, Value};
use swc_core::{common::comments, ecma::ast};

use crate::utils::swc::{clean_comment, get_decorator};

use super::types::{
	Playground, PlaygroundAction, PlaygroundInput, PlaygroundInputConfig, PlaygroundInputConfigExt,
	PlaygroundInputType,
};

pub(crate) fn parse_playground<C: comments::Comments>(
	node: &ast::Class,
	comments: &C,
) -> Result<Option<Playground>> {
	let inputs = extract_inputs(node, comments)?;
	let actions = extract_actions(node, comments);

	if actions.is_empty() && inputs.is_empty() {
		Ok(None)
	} else {
		Ok(Some(Playground { inputs, actions }))
	}
}

fn extract_inputs<C: comments::Comments>(
	node: &ast::Class,
	comments: &C,
) -> Result<Vec<PlaygroundInput>> {
	let mut result = Vec::new();

	for member in &node.body {
		let	(ast::ClassMember::AutoAccessor(ast::AutoAccessor {
				key: ast::Key::Public(key),
				decorators,
				value,
				..
			})
			| ast::ClassMember::ClassProp(ast::ClassProp {
				key,
				decorators,
				value,
				..
			})) = member else { continue };

		if get_decorator(decorators, "Input").is_none() {
			continue;
		}

		let Some(name) = to_name(key) else { continue };

		let mut description: Option<String> = None;
		let mut config: Option<PlaygroundInputConfig> = None;

		if let Some(comment) = get_leading_comment(comments, member) {
			let comment = clean_comment(&comment);

			let mut parts = comment.splitn(2, "@input");
			description = parts.next().map(ToString::to_string);

			if let Some(default) = parts.next() {
				config = Some(serde_json::from_str(default)?);
			}
		}

		let config = config.extend(
			value
				.as_ref()
				.and_then(evaluate)
				.unwrap_or(PlaygroundInputConfig::default()),
		);

		result.push(PlaygroundInput {
			name: name.to_owned(),
			description,
			config,
		});
	}

	Ok(result)
}

fn extract_actions<C: comments::Comments>(
	node: &ast::Class,
	comments: &C,
) -> Vec<PlaygroundAction> {
	node.body
		.iter()
		.filter_map(ast::ClassMember::as_method)
		.filter_map(|method| -> Option<PlaygroundAction> {
			let comment = get_leading_comment(comments, method)?;

			if comment.text.contains("@action") {
				let name = to_name(&method.key)?.to_owned();
				Some(PlaygroundAction {
					name,
					description: clean_comment(&comment).replace("@action", ""),
				})
			} else {
				None
			}
		})
		.collect()
}

#[inline]
fn to_name(prop_name: &ast::PropName) -> Option<&str> {
	match prop_name {
		ast::PropName::Ident(ast::Ident { sym, .. }) => Some(sym.as_ref()),
		ast::PropName::Str(ast::Str { value, .. }) => Some(value.as_ref()),
		_ => None,
	}
}

#[inline]
fn get_leading_comment<T: comments::Comments, N: swc_core::common::Spanned>(
	comments: &T,
	node: &N,
) -> Option<comments::Comment> {
	comments
		.get_leading(node.span_lo())
		.and_then(|comments| comments.into_iter().next())
}

fn evaluate<T: AsRef<ast::Expr>>(expr: T) -> Option<PlaygroundInputConfig> {
	match expr.as_ref() {
		ast::Expr::Lit(ast::Lit::Bool(value)) => Some(PlaygroundInputConfig::from_default(
			Value::Bool(value.value),
		)),
		ast::Expr::Lit(ast::Lit::Num(value)) => Some(PlaygroundInputConfig::new(
			Number::from_f64(value.value).map(Value::Number),
			PlaygroundInputType::Number,
		)),
		ast::Expr::Lit(ast::Lit::Str(value)) => Some(PlaygroundInputConfig::from_default(
			Value::String(value.value.to_string()),
		)),

		ast::Expr::Tpl(_) => Some(PlaygroundInputConfig::from_type(
			PlaygroundInputType::String,
		)),

		ast::Expr::TsAs(ast::TsAsExpr { expr, .. })
		| ast::Expr::TsNonNull(ast::TsNonNullExpr { expr, .. })
		| ast::Expr::TsSatisfies(ast::TsSatisfiesExpr { expr, .. })
		| ast::Expr::TsTypeAssertion(ast::TsTypeAssertion { expr, .. }) => evaluate(expr),

		_ => None,
	}
}
