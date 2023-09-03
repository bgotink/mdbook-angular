use serde_json::{Number, Value};
use swc_core::ecma::ast;

use super::{PlaygroundInputConfig, PlaygroundInputConfigExt, PlaygroundInputType};

fn apply_unary(
	op: ast::UnaryOp,
	arg: Option<PlaygroundInputConfig>,
) -> Option<PlaygroundInputConfig> {
	match op {
		ast::UnaryOp::Minus => {
			if let Some(value) = arg.into_number() {
				let new_value = if let Some(val) = value.as_i64() {
					Number::from(-val)
				} else {
					Number::from_f64(-value.as_f64().unwrap()).unwrap()
				};

				Some(PlaygroundInputConfig::from_default(new_value))
			} else {
				Some(PlaygroundInputConfig::number())
			}
		}
		ast::UnaryOp::Plus => {
			if let Some(value) = arg.into_number() {
				Some(PlaygroundInputConfig::from_default(value))
			} else {
				Some(PlaygroundInputConfig::number())
			}
		}

		ast::UnaryOp::Bang => {
			if let Some(value) = arg.as_boolean() {
				Some(PlaygroundInputConfig::from_default(!value))
			} else {
				Some(PlaygroundInputConfig::boolean())
			}
		}

		ast::UnaryOp::Tilde => Some(PlaygroundInputConfig::number()),
		ast::UnaryOp::TypeOf => Some(PlaygroundInputConfig::string()),

		_ => None,
	}
}

#[allow(clippy::too_many_lines)]
fn apply_binary(
	op: ast::BinaryOp,
	left: &Option<PlaygroundInputConfig>,
	right: &Option<PlaygroundInputConfig>,
) -> Option<PlaygroundInputConfig> {
	fn values<'a>(
		left: &'a Option<PlaygroundInputConfig>,
		right: &'a Option<PlaygroundInputConfig>,
	) -> Option<(&'a Value, &'a Value)> {
		Option::zip(left.get_default(), right.get_default())
	}

	match op {
		ast::BinaryOp::NotEq
		| ast::BinaryOp::In
		| ast::BinaryOp::InstanceOf
		| ast::BinaryOp::EqEq => Some(PlaygroundInputConfig::boolean()),

		ast::BinaryOp::EqEqEq => Some(if let Some((left, right)) = values(left, right) {
			PlaygroundInputConfig::from_default(left.eq(right))
		} else {
			PlaygroundInputConfig::boolean()
		}),
		ast::BinaryOp::NotEqEq => Some(if let Some((left, right)) = values(left, right) {
			PlaygroundInputConfig::from_default(!left.eq(right))
		} else {
			PlaygroundInputConfig::boolean()
		}),

		ast::BinaryOp::LogicalAnd => Some(if let Some(left) = left.as_boolean() {
			if !left {
				PlaygroundInputConfig::from_default(false)
			} else if let Some(right) = right.as_boolean() {
				PlaygroundInputConfig::from_default(right)
			} else {
				PlaygroundInputConfig::boolean()
			}
		} else {
			PlaygroundInputConfig::boolean()
		}),
		ast::BinaryOp::LogicalOr => Some(if let Some(left) = left.as_boolean() {
			if left {
				PlaygroundInputConfig::from_default(true)
			} else if let Some(right) = right.as_boolean() {
				PlaygroundInputConfig::from_default(right)
			} else {
				PlaygroundInputConfig::boolean()
			}
		} else {
			PlaygroundInputConfig::boolean()
		}),

		ast::BinaryOp::BitAnd => {
			i64_i64_to_i64_operator(values(left, right), std::ops::BitAnd::bitand)
		}
		ast::BinaryOp::BitOr => {
			i64_i64_to_i64_operator(values(left, right), std::ops::BitOr::bitor)
		}
		ast::BinaryOp::BitXor => {
			i64_i64_to_i64_operator(values(left, right), std::ops::BitXor::bitxor)
		}
		ast::BinaryOp::LShift => i64_i64_to_i64_operator(values(left, right), std::ops::Shl::shl),
		ast::BinaryOp::RShift => i64_i64_to_i64_operator(values(left, right), std::ops::Shr::shr),
		ast::BinaryOp::ZeroFillRShift => Some(PlaygroundInputConfig::number()),

		ast::BinaryOp::Exp => f64_f64_to_f64_operator(values(left, right), f64::powf),
		ast::BinaryOp::Mod => f64_f64_to_f64_operator(values(left, right), std::ops::Rem::rem),
		ast::BinaryOp::Mul => f64_f64_to_f64_operator(values(left, right), std::ops::Mul::mul),
		ast::BinaryOp::Div => f64_f64_to_f64_operator(values(left, right), std::ops::Div::div),
		ast::BinaryOp::Sub => f64_f64_to_f64_operator(values(left, right), std::ops::Sub::sub),

		ast::BinaryOp::Gt => f64_f64_to_bool_operator(values(left, right), |a, b| a > b),
		ast::BinaryOp::GtEq => f64_f64_to_bool_operator(values(left, right), |a, b| a >= b),
		ast::BinaryOp::Lt => f64_f64_to_bool_operator(values(left, right), |a, b| a < b),
		ast::BinaryOp::LtEq => f64_f64_to_bool_operator(values(left, right), |a, b| a <= b),

		ast::BinaryOp::Add => {
			// Oh boy
			if let Some((left, right)) = values(left, right) {
				match (left, right) {
					(Value::String(left), Value::String(right)) => {
						let mut result = String::with_capacity(left.len() + right.len());
						result.push_str(left);
						result.push_str(right);

						Some(PlaygroundInputConfig::from_default(result))
					}
					(Value::String(_), _) | (_, Value::String(_)) => {
						Some(PlaygroundInputConfig::string())
					}

					(Value::Number(left), Value::Number(right)) => Some(
						if let Some(result) = serde_json::Number::from_f64(
							left.as_f64().unwrap() + right.as_f64().unwrap(),
						) {
							PlaygroundInputConfig::from_default(result)
						} else {
							PlaygroundInputConfig::number()
						},
					),
					(Value::Bool(true), Value::Number(num))
					| (Value::Number(num), Value::Bool(true)) => Some(
						if let Some(result) =
							serde_json::Number::from_f64(num.as_f64().unwrap() + 1.0)
						{
							PlaygroundInputConfig::from_default(result)
						} else {
							PlaygroundInputConfig::number()
						},
					),

					(Value::Number(num), Value::Null | Value::Bool(false))
					| (Value::Null | Value::Bool(false), Value::Number(num)) => {
						Some(PlaygroundInputConfig::from_default(num.clone()))
					}

					_ => None,
				}
			} else {
				None
			}
		}

		ast::BinaryOp::NullishCoalescing => None,
	}
}

#[allow(clippy::unnecessary_wraps)]
fn f64_f64_to_f64_operator<F>(
	values: Option<(&Value, &Value)>,
	f: F,
) -> Option<PlaygroundInputConfig>
where
	F: Fn(f64, f64) -> f64,
{
	if let Some((Value::Number(left), Value::Number(right))) = values {
		if let Some(value) =
			serde_json::Number::from_f64(f(left.as_f64().unwrap(), right.as_f64().unwrap()))
		{
			Some(PlaygroundInputConfig::from_default(value))
		} else {
			Some(PlaygroundInputConfig::number())
		}
	} else {
		Some(PlaygroundInputConfig::number())
	}
}

#[allow(clippy::unnecessary_wraps)]
fn f64_f64_to_bool_operator<F>(
	values: Option<(&Value, &Value)>,
	f: F,
) -> Option<PlaygroundInputConfig>
where
	F: Fn(f64, f64) -> bool,
{
	if let Some((Value::Number(left), Value::Number(right))) = values {
		Some(PlaygroundInputConfig::from_default(f(
			left.as_f64().unwrap(),
			right.as_f64().unwrap(),
		)))
	} else {
		Some(PlaygroundInputConfig::boolean())
	}
}

#[allow(clippy::unnecessary_wraps)]
fn i64_i64_to_i64_operator<F>(
	values: Option<(&Value, &Value)>,
	f: F,
) -> Option<PlaygroundInputConfig>
where
	F: Fn(i64, i64) -> i64,
{
	if let Some((Value::Number(left), Value::Number(right))) = values {
		if let Some(left) = left.as_i64() {
			if let Some(right) = right.as_i64() {
				return Some(PlaygroundInputConfig::from_default(f(left, right)));
			}
		}
	}

	Some(PlaygroundInputConfig::number())
}

pub(super) fn ts_type_to_input_type<T: AsRef<ast::TsType>>(
	ts_type: &T,
) -> Option<PlaygroundInputType> {
	match ts_type.as_ref() {
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

pub(super) fn evaluate<T: AsRef<ast::Expr>>(expr: T) -> Option<PlaygroundInputConfig> {
	match expr.as_ref() {
		ast::Expr::Lit(ast::Lit::Bool(value)) => {
			Some(PlaygroundInputConfig::from_default(value.value))
		}
		ast::Expr::Lit(ast::Lit::Num(value)) => Some(PlaygroundInputConfig::new(
			Number::from_f64(value.value).map(Value::Number),
			PlaygroundInputType::Number,
		)),
		ast::Expr::Lit(ast::Lit::Str(value)) => {
			Some(PlaygroundInputConfig::from_default(value.value.to_string()))
		}

		ast::Expr::Tpl(_) => Some(PlaygroundInputConfig::string()),

		ast::Expr::Unary(ast::UnaryExpr { op, arg, .. }) => apply_unary(*op, evaluate(arg)),

		ast::Expr::TsNonNull(ast::TsNonNullExpr { expr, .. }) => evaluate(expr),

		ast::Expr::TsAs(ast::TsAsExpr { expr, type_ann, .. })
		| ast::Expr::TsSatisfies(ast::TsSatisfiesExpr { expr, type_ann, .. })
		| ast::Expr::TsTypeAssertion(ast::TsTypeAssertion { expr, type_ann, .. }) => evaluate(expr)
			.or_else(|| ts_type_to_input_type(type_ann).map(PlaygroundInputConfig::from_type)),

		ast::Expr::Bin(ast::BinExpr {
			op, left, right, ..
		}) => apply_binary(*op, &evaluate(left), &evaluate(right)),

		_ => None,
	}
}
