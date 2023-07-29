use std::path::{Path, PathBuf};

use anyhow::Result;
use mdbook::renderer::RenderContext;
use toml::Value;

#[allow(clippy::struct_excessive_bools)] // this is config, not a state machine
pub(crate) struct Config {
	pub(crate) background: bool,
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
	pub(super) fn read<P: AsRef<Path>>(root: P) -> Result<Self> {
		let root = root.as_ref();
		let mut cfg = mdbook::Config::from_disk(root.join("book.toml"))?;
		cfg.update_from_env();

		Ok(Self::from_config(
			&cfg,
			root,
			// Incorrect if there are multiple backends, but... good enough?
			root.join(&cfg.build.build_dir),
		))
	}

	pub(super) fn new(ctx: &RenderContext) -> Self {
		Self::from_config(&ctx.config, &ctx.root, ctx.destination.clone())
	}

	fn from_config(config: &mdbook::Config, root: &Path, destination: PathBuf) -> Self {
		let experimental_builder = config
			.get("output.angular.experimental-builder")
			.and_then(Value::as_bool)
			.unwrap_or(true);

		let background = config
			.get("output.angular.background")
			.and_then(Value::as_bool)
			.unwrap_or(false);

		let playgrounds = config
			.get("output.angular.playgrounds")
			.and_then(Value::as_bool)
			.unwrap_or(true);

		let tsconfig = config
			.get("output.angular.tsconfig")
			.and_then(Value::as_str)
			.map(|tsconfig| root.join(tsconfig));

		let inline_style_language = config
			.get("output.angular.inline-style-language")
			.and_then(Value::as_str)
			.unwrap_or("css")
			.to_owned();

		let optimize = config
			.get("output.angular.optimize")
			.and_then(Value::as_bool)
			.unwrap_or(false);

		let book_source_folder = root.join(&config.book.src);

		let angular_root_folder = root.join(
			config
				.get("output.angular.workdir")
				.and_then(Value::as_str)
				.unwrap_or("mdbook_angular"),
		);

		let target_folder = destination;

		Config {
			background,
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
