use swc_core::{common::comments, ecma::ast};

use crate::{
	utils::swc::{clean_comment, get_decorator},
	Result,
};

use super::{
	evaluate_expression::{evaluate, ts_type_to_input_type},
	types::{
		Playground, PlaygroundAction, PlaygroundInput, PlaygroundInputConfig,
		PlaygroundInputConfigExt, PlaygroundInputType,
	},
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
				type_ann
					.as_deref()
					.and_then(|ann| ts_type_to_input_type(&ann.type_ann)),
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
					.first()
					.and_then(|param| extract_type_from_pat(&param.pat)),
			),

			_ => continue,
		};

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

		if let Some(input_decorator) = get_decorator(decorators, "Input") {
			let Some(name) = get_name_from_input_decorator(input_decorator)
				.or_else(|| to_name(key).map(ToOwned::to_owned))
			else {
				continue;
			};

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
				name,
				description,
				config,
			});
		} else if let Some(call) = value.as_ref().and_then(|value| value.as_call()) {
			if call.callee.is_expr() && call.callee.as_expr().unwrap().is_ident_ref_to("input") {
				let value = call.args.first().map(|v| &v.expr);

				let Some(name) = get_name_from_input_signal(call)
					.or_else(|| to_name(key).map(ToOwned::to_owned))
				else {
					continue;
				};

				if let Some(type_) = call
					.type_args
					.as_ref()
					.and_then(|type_args| type_args.params.first())
					.and_then(ts_type_to_input_type)
				{
					config = Some(config.extend(PlaygroundInputConfig::from_type(type_)));
				}

				let config = config.extend(
					value
						.and_then(evaluate)
						.unwrap_or(PlaygroundInputConfig::default()),
				);

				result.push(PlaygroundInput {
					name,
					description,
					config,
				});
			}
		}
	}

	Ok(result)
}

fn get_name_from_input_decorator(decorator: &ast::Decorator) -> Option<String> {
	let arg = decorator
		.expr
		.as_call()
		.and_then(|call| call.args.first())?;

	if let Some(ast::Lit::Str(str)) = arg.expr.as_lit() {
		return Some(str.value.as_str().to_owned());
	}

	let alias = arg
		.expr
		.as_object()?
		.props
		.iter()
		.filter_map(|prop| prop.as_prop())
		.filter_map(|prop| prop.as_key_value())
		.find(|prop| prop.key.is_str() && prop.key.as_str().unwrap().value.eq("alias"))?;

	let ast::Lit::Str(alias) = alias.value.as_lit()? else {
		return None;
	};

	Some(alias.value.as_str().to_owned())
}

fn get_name_from_input_signal(call: &ast::CallExpr) -> Option<String> {
	let opts = call.args.get(1)?;
	let opts = opts.expr.as_object()?;

	let alias = opts
		.props
		.iter()
		.filter_map(|prop| prop.as_prop())
		.filter_map(|prop| prop.as_key_value())
		.find(|prop| prop.key.is_str() && prop.key.as_str().unwrap().value.eq("alias"))?;

	let ast::Lit::Str(alias) = alias.value.as_lit()? else {
		return None;
	};

	Some(alias.value.as_str().to_owned())
}

fn extract_type_from_pat(pat: &ast::Pat) -> Option<PlaygroundInputType> {
	match pat {
		ast::Pat::Object(ast::ObjectPat { type_ann, .. })
		| ast::Pat::Ident(ast::BindingIdent { type_ann, .. })
		| ast::Pat::Array(ast::ArrayPat { type_ann, .. }) => type_ann
			.as_deref()
			.and_then(|ann| ts_type_to_input_type(&ann.type_ann)),

		ast::Pat::Assign(ast::AssignPat { left, right, .. }) => extract_type_from_pat(left)
			.or_else(|| evaluate(right).map(PlaygroundInputConfig::get_type)),

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

fn to_name(prop_name: &ast::PropName) -> Option<&str> {
	match prop_name {
		ast::PropName::Ident(ast::IdentName { sym, .. }) => Some(sym.as_ref()),
		ast::PropName::Str(ast::Str { value, .. }) => Some(value.as_ref()),
		_ => None,
	}
}

fn get_leading_comment<T: comments::Comments, N: swc_core::common::Spanned>(
	comments: &T,
	node: &N,
) -> Option<comments::Comment> {
	comments
		.get_leading(node.span_lo())
		.and_then(|comments| comments.into_iter().next())
}
