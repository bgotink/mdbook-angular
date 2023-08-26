use serde_json::{Number, Value};
use swc_core::{common::comments, ecma::ast};

use crate::{
	utils::swc::{clean_comment, get_decorator},
	Result,
};

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
		let (key, decorators, value, type_) = match member {
			ast::ClassMember::AutoAccessor(ast::AutoAccessor {
				key: ast::Key::Public(key),
				decorators,
				value,
				type_ann,
				..
			})
			| ast::ClassMember::ClassProp(ast::ClassProp {
				key,
				decorators,
				value,
				type_ann,
				..
			}) => (
				key,
				decorators,
				value,
				type_ann.as_deref().and_then(type_ann_to_input_type),
			),

			ast::ClassMember::Method(ast::ClassMethod {
				kind: ast::MethodKind::Setter,
				key,
				function,
				..
			}) => (
				key,
				&function.decorators,
				&None,
				function
					.params
					.get(0)
					.and_then(|param| extract_type_from_pat(&param.pat)),
			),

			_ => continue,
		};

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

		if let Some(type_) = type_ {
			config = Some(config.extend(PlaygroundInputConfig::from_type(type_)));
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

fn extract_type_from_pat(pat: &ast::Pat) -> Option<PlaygroundInputType> {
	match pat {
		ast::Pat::Object(ast::ObjectPat { type_ann, .. })
		| ast::Pat::Ident(ast::BindingIdent { type_ann, .. })
		| ast::Pat::Array(ast::ArrayPat { type_ann, .. }) => {
			type_ann.as_deref().and_then(type_ann_to_input_type)
		}

		ast::Pat::Assign(ast::AssignPat { left, right, .. }) => extract_type_from_pat(left)
			.or_else(|| evaluate(right).map(PlaygroundInputConfig::get_type)),

		_ => None,
	}
}

fn type_ann_to_input_type(type_ann: &ast::TsTypeAnn) -> Option<PlaygroundInputType> {
	match type_ann.type_ann.as_ref() {
		ast::TsType::TsKeywordType(ast::TsKeywordType {
			kind: ast::TsKeywordTypeKind::TsNumberKeyword,
			..
		}) => Some(PlaygroundInputType::Number),
		ast::TsType::TsKeywordType(ast::TsKeywordType {
			kind: ast::TsKeywordTypeKind::TsStringKeyword,
			..
		}) => Some(PlaygroundInputType::String),
		ast::TsType::TsKeywordType(ast::TsKeywordType {
			kind: ast::TsKeywordTypeKind::TsBooleanKeyword,
			..
		}) => Some(PlaygroundInputType::Boolean),

		ast::TsType::TsUnionOrIntersectionType(ast::TsUnionOrIntersectionType::TsUnionType(
			ast::TsUnionType { types, .. },
		)) => {
			let string_types: Vec<_> = types
				.iter()
				.filter_map(|t| t.as_ts_lit_type())
				.filter_map(|t| t.lit.as_str())
				.map(|v| v.value.to_string())
				.collect();

			if string_types.len() == types.len() {
				Some(PlaygroundInputType::Enum(string_types))
			} else {
				None
			}
		}
		_ => None,
	}
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

		ast::Expr::Unary(ast::UnaryExpr { op, arg, .. }) => match op {
			ast::UnaryOp::Minus => {
				if let Some(Value::Number(num)) =
					evaluate(arg).and_then(PlaygroundInputConfig::get_default)
				{
					let val = if let Some(val) = num.as_i64() {
						Value::Number(Number::from(-val))
					} else {
						Value::Number(Number::from_f64(-num.as_f64().unwrap()).unwrap())
					};

					Some(PlaygroundInputConfig::from_default(val))
				} else {
					Some(PlaygroundInputConfig::from_type(
						PlaygroundInputType::Number,
					))
				}
			}

			ast::UnaryOp::Plus => {
				let mut cfg = PlaygroundInputConfig::from_type(PlaygroundInputType::Number);

				if let Some(value_cfg) = evaluate(arg) {
					if let Some(Value::Number(num)) = value_cfg.get_default() {
						cfg = PlaygroundInputConfig::from_default(Value::Number(num));
					}
				}

				Some(cfg)
			}

			ast::UnaryOp::Bang => Some(PlaygroundInputConfig::from_type(
				PlaygroundInputType::Boolean,
			)),
			ast::UnaryOp::Tilde => Some(PlaygroundInputConfig::from_type(
				PlaygroundInputType::Number,
			)),
			ast::UnaryOp::TypeOf => Some(PlaygroundInputConfig::from_type(
				PlaygroundInputType::String,
			)),

			_ => None,
		},

		ast::Expr::TsAs(ast::TsAsExpr { expr, .. })
		| ast::Expr::TsNonNull(ast::TsNonNullExpr { expr, .. })
		| ast::Expr::TsSatisfies(ast::TsSatisfiesExpr { expr, .. })
		| ast::Expr::TsTypeAssertion(ast::TsTypeAssertion { expr, .. }) => evaluate(expr),

		_ => None,
	}
}
