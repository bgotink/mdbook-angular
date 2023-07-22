use std::path::PathBuf;

use mdbook::renderer::RenderContext;
use toml::Value;

pub(crate) struct Config {
	pub(crate) experimental_builder: bool,
	pub(crate) playgrounds: bool,
	pub(crate) tsconfig: Option<PathBuf>,
	pub(crate) inline_style_language: String,
	pub(crate) optimize: bool,

	pub(crate) book_source_folder: PathBuf,
	pub(crate) angular_root_folder: PathBuf,
	pub(crate) target_folder: PathBuf,
}

impl Config {
	pub(super) fn new(ctx: &RenderContext) -> Self {
		let experimental_builder = ctx
			.config
			.get("output.angular.experimentalBuilder")
			.and_then(Value::as_bool)
			.unwrap_or(true);

		let playgrounds = ctx
			.config
			.get("output.angular.playgrounds")
			.and_then(Value::as_bool)
			.unwrap_or(true);

		let tsconfig = ctx
			.config
			.get("output.angular.tsconfig")
			.and_then(Value::as_str)
			.map(|tsconfig| ctx.root.join(tsconfig));

		let inline_style_language = ctx
			.config
			.get("output.angular.inlineStyleLanguage")
			.and_then(Value::as_str)
			.unwrap_or("css")
			.to_owned();

		let optimize = ctx
			.config
			.get("output.angular.optimize")
			.and_then(Value::as_bool)
			.unwrap_or(false);

		let book_source_folder = ctx.source_dir();

		let angular_root_folder = ctx.root.join(
			ctx.config
				.get("output.angular.workdir")
				.and_then(Value::as_str)
				.unwrap_or("mdbook_angular"),
		);

		let target_folder = ctx.destination.clone();

		Config {
			experimental_builder,
			playgrounds,
			tsconfig,
			inline_style_language,
			optimize,

			book_source_folder,
			angular_root_folder,
			target_folder,
		}
	}
}
