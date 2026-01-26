use std::{borrow::Cow, collections::VecDeque, sync::LazyLock};

use regex::Regex;
use swc_core::{
	common::comments::{self, CommentKind},
	ecma::ast,
};

pub(crate) fn clean_comment(comment: &comments::Comment) -> String {
	static COMMENT_BLOCK_LINE_START: LazyLock<Regex> =
		LazyLock::new(|| Regex::new(r"^\s*\*\s?").unwrap());
	static LINE_COMMENT_START: LazyLock<Regex> =
		LazyLock::new(|| Regex::new(r"^\s*//\s?").unwrap());

	let start_of_line_regex = match comment.kind {
		CommentKind::Block => &COMMENT_BLOCK_LINE_START,
		CommentKind::Line => &LINE_COMMENT_START,
	};

	let mut lines = comment
		.text
		.lines()
		.map(|line| {
			let line = start_of_line_regex.replace(line, "");

			if line.trim().is_empty() {
				Cow::Borrowed("")
			} else {
				line
			}
		})
		.collect::<VecDeque<_>>();

	if let Some(first) = lines.front() {
		if first.is_empty() {
			lines.pop_front();
		}
	}

	if let Some(last) = lines.back() {
		if last.is_empty() {
			lines.pop_back();
		}
	}

	lines.into_iter().collect::<Vec<_>>().join("\n")
}

pub(crate) fn get_decorator<'a>(
	decorators: &'a [ast::Decorator],
	name: &str,
) -> Option<&'a ast::Decorator> {
	decorators.iter().find(|decorator| {
		decorator
			.expr
			.as_call()
			.and_then(|call| call.callee.as_expr())
			.and_then(|callee| callee.as_ident())
			.is_some_and(|ident| ident.sym.as_ref() == name)
	})
}
