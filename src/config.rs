use std::path::{Path, PathBuf};

use mdbook::renderer::RenderContext;
use toml::Value;

use crate::{Error, Result};

/// Configuration for mdbook-angular
#[allow(clippy::struct_excessive_bools)] // this is config, not a state machine
pub struct Config {
	/// Whether to enable the experimental background builder
	///
	/// Enabling this option runs the angular build in a background process,
	/// triggering a watch instead of an entire new build whenever mdbook notices
	/// a change.
	/// This is considerably faster.
	///
	/// This option requires the [`Config.experimental_builder`] option to be enabled.
	/// It only works on builds with the "background" feature enabled, and it only
	/// works on platforms rustc considers "unix".
	/// This option is no-op for commands that don't watch the book source for
	/// changes.
	///
	/// Default value: `false`
	pub background: bool,
	/// Whether to use an experimental builder (requires angular ≥ 16.2.0)
	///
	/// If enabled, all chapters in the book will be built in a single go. If
	/// disabled, every chapter is built separately as angular application.
	///
	/// Default value: `false`
	pub experimental_builder: bool,
	/// Whether playgrounds are enabled by default
	///
	/// This can be overridden via `playground` or `no-playground` tag on every
	/// individual code block or `{{#angular}}` tag.
	///
	/// Default value: `true`
	pub playgrounds: bool,
	/// Path to a tsconfig to use for building, relative to the `book.toml` file
	pub tsconfig: Option<PathBuf>,
	/// The inline style language the angular compiler should use
	///
	/// Default value: `"css"`
	pub inline_style_language: String,
	/// Whether to optimize the angular applications
	///
	/// This option is ignored if background is active
	///
	/// Default value: `false`
	pub optimize: bool,
	/// Polyfills to import, if any
	///
	/// Note: zone.js is always included as polyfill.
	///
	/// This only supports bare specifiers, you can't add relative imports here.
	pub polyfills: Vec<String>,

	pub(crate) book_source_folder: PathBuf,
	pub(crate) angular_root_folder: PathBuf,
	pub(crate) target_folder: PathBuf,
}

impl Config {
	/// Read mdbook-angular [`Config`] from the `book.toml` file inside the given folder.
	///
	/// # Errors
	///
	/// This function will return an error if reading the `book.toml` fails or if
	/// the book contains an invalid configuration.
	pub fn read<P: AsRef<Path>>(root: P) -> Result<Self> {
		let root = root.as_ref();
		let mut cfg = mdbook::Config::from_disk(root.join("book.toml"))?;
		cfg.update_from_env();

		Self::from_config(
			&cfg,
			root,
			// Incorrect if there are multiple backends, but... good enough?
			root.join(&cfg.build.build_dir),
		)
	}

	/// Create mdbook-angular configuration [`Config`] from the given render context.
	///
	/// # Errors
	///
	/// This function fails if the context contains an invalid configuration.
	pub fn new(ctx: &RenderContext) -> Result<Self> {
		Self::from_config(&ctx.config, &ctx.root, ctx.destination.clone())
	}

	fn from_config(config: &mdbook::Config, root: &Path, destination: PathBuf) -> Result<Self> {
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

		let polyfills = config
			.get("output.angular.polyfills")
			.and_then(Value::as_array)
			.map(|polyfills| {
				polyfills
					.iter()
					.map(|value| value.as_str().map(ToOwned::to_owned))
					.collect::<Option<Vec<_>>>()
					.ok_or(Error::msg(
						"Invalid polyfills, expected an array of strings",
					))
			})
			.transpose()?
			.unwrap_or_default();

		let book_source_folder = root.join(&config.book.src);

		let angular_root_folder = root.join(
			config
				.get("output.angular.workdir")
				.and_then(Value::as_str)
				.unwrap_or("mdbook_angular"),
		);

		let target_folder = destination;

		Ok(Config {
			background,
			experimental_builder,
			playgrounds,
			tsconfig,
			inline_style_language,
			optimize,
			polyfills,

			book_source_folder,
			angular_root_folder,
			target_folder,
		})
	}
}
