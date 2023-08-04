use std::path::{Path, PathBuf};

use anyhow::Context;
use mdbook::renderer::RenderContext;
use serde::Deserialize;
use toml::Table;

use crate::Result;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct DeConfig {
	#[allow(unused)] // the command option is defined by mdbook
	command: Option<String>,

	background: Option<bool>,
	experimental_builder: Option<bool>,
	collapsed: Option<bool>,
	playgrounds: Option<bool>,
	tsconfig: Option<PathBuf>,
	inline_style_language: Option<String>,
	optimize: Option<bool>,
	polyfills: Option<Vec<String>>,
	workdir: Option<String>,
}

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
	/// Whether code blocks should be collapsed by default
	///
	/// This can be overridden via `collapsed` or `uncollapsed` tag on every
	/// individual code block or `{{#angular}}` tag
	///
	/// Note this only takes effect on code blocks tagged with "angular", it
	/// doesn't affect other code blocks.
	///
	/// Default value: `false`
	pub collapsed: bool,
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
		let mut cfg =
			mdbook::Config::from_disk(root.join("book.toml")).context("Error reading book.toml")?;
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
		let de_config: DeConfig = config
			.get_renderer("angular")
			.map_or_else(Table::default, ToOwned::to_owned)
			.try_into()
			.context("Failed to parse mdbook-angular configuration")?;

		let book_source_folder = root.join(&config.book.src);

		let angular_root_folder =
			PathBuf::from(de_config.workdir.unwrap_or("mdbook_angular".to_owned()));
		let angular_root_folder = if angular_root_folder.is_absolute() {
			angular_root_folder
		} else {
			root.join(angular_root_folder)
		};

		let target_folder = destination;

		Ok(Config {
			experimental_builder: de_config.experimental_builder.unwrap_or(true),
			background: de_config.background.unwrap_or(false),
			collapsed: de_config.collapsed.unwrap_or(false),
			playgrounds: de_config.playgrounds.unwrap_or(true),
			tsconfig: de_config.tsconfig.map(|tsconfig| root.join(tsconfig)),
			inline_style_language: de_config.inline_style_language.unwrap_or("css".to_owned()),
			optimize: de_config.optimize.unwrap_or(false),
			polyfills: de_config.polyfills.unwrap_or_default(),

			book_source_folder,
			angular_root_folder,
			target_folder,
		})
	}
}
