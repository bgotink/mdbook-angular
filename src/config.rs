use std::path::{Path, PathBuf};

use anyhow::Context;
use mdbook::renderer::RenderContext;
use serde::Deserialize;
use toml::value::Table;

use crate::Result;

#[derive(Deserialize, PartialEq, Eq, Debug, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Builder {
	/// Build all chapters in a single angular build.
	///
	/// This is fast, but uses internal Angular APIs to use the currently
	/// experimental "application" builder Angular provides as of 16.2.0.
	#[default]
	Experimental,
	/// Build via [`Builder::Experimental`] in a background process.
	///
	/// This allows the angular process to keep running after the renderer exits.
	/// This builder option enables watching, which significantly speeds up
	/// rebuilds.
	///
	/// This option is not supported on Windows, where this option is considered
	/// an alias to [`Builder::Experimental`].
	Background,
	/// Build every chapter as a separate angular application.
	///
	/// This uses stable Angular APIs and should work for Angular 14.0.0 and up.
	Slow,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct DeConfig {
	#[allow(unused)] // the command option is defined by mdbook
	command: Option<String>,

	#[serde(default)]
	builder: Builder,
	collapsed: Option<bool>,
	playgrounds: Option<bool>,
	tsconfig: Option<PathBuf>,
	inline_style_language: Option<String>,
	optimize: Option<bool>,
	polyfills: Option<Vec<String>>,
	workdir: Option<String>,

	html: Option<Table>,
}

/// Configuration for mdbook-angular
pub struct Config {
	/// Builder to use to compile the angular code
	///
	/// Default value: [`Builder::Experimental`]
	pub builder: Builder,
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

	/// Configuration to pass to the HTML renderer
	///
	/// Use this intead of the `output.html` table itself to configure the HTML
	/// renderer without having mdbook run the HTML renderer standalone.
	pub html: Option<Table>,

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
		let angular_renderer_config = config
			.get_renderer("angular")
			.map_or_else(Default::default, ToOwned::to_owned);
		let de_config: DeConfig = toml::Value::from(angular_renderer_config)
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
			builder: de_config.builder,
			collapsed: de_config.collapsed.unwrap_or(false),
			playgrounds: de_config.playgrounds.unwrap_or(true),
			tsconfig: de_config.tsconfig.map(|tsconfig| root.join(tsconfig)),
			inline_style_language: de_config.inline_style_language.unwrap_or("css".to_owned()),
			optimize: de_config.optimize.unwrap_or(false),
			polyfills: de_config.polyfills.unwrap_or_default(),

			html: de_config.html,

			book_source_folder,
			angular_root_folder,
			target_folder,
		})
	}
}
