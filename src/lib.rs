#![doc = include_str!("../README.md")]

mod angular;
pub(crate) mod codeblock;
pub(crate) mod config;
mod js;
mod markdown;
mod utils;

/// The version of mdbook-angular
pub const MDBOOK_ANGULAR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The expected version of mdbook
///
/// This crate can be used with any mdbook version that are semver compatible
/// with this expected version.
pub const EXPECTED_MDBOOK_VERSION: &str = mdbook::MDBOOK_VERSION;

pub use angular::stop_background_process;
pub use config::Config;

use angular::build;
use log::debug;
use markdown::process_markdown;
use markdown::ChapterWithCodeBlocks;
use mdbook::{
	renderer::{HtmlHandlebars, RenderContext},
	BookItem, Renderer,
};

fn validate_version(ctx: &RenderContext) -> Result<()> {
	let req = semver::VersionReq::parse(EXPECTED_MDBOOK_VERSION).unwrap();

	if semver::Version::parse(&ctx.version).map_or(false, |version| req.matches(&version)) {
		Ok(())
	} else {
		bail!("Invalid mdbook version {}, expected {}", &ctx.version, req);
	}
}

pub(crate) use anyhow::{bail, Context, Error, Result};

/// An mdbook [`Renderer`] for including live angular code samples
pub struct AngularRenderer {}

impl Renderer for AngularRenderer {
	fn name(&self) -> &str {
		"angular"
	}

	/// Prefer [`Self::render_mut`]
	///
	/// The [`AngularRenderer`] has to modify the book passed in with the context,
	/// so this function has to clone the given context in order to mutate it.
	/// Using [`Self::render_mut`] can prevent a needless copy.
	#[inline]
	fn render(&self, ctx: &RenderContext) -> Result<()> {
		self.render_mut(&mut ctx.clone())
	}
}

impl AngularRenderer {
	pub fn new() -> Self {
		Self {}
	}

	/// Renders the given [`RenderContext`]
	///
	/// This function can make changes to the context, notably to edit the markdown
	/// files to insert angular code blocks, live angular applications, and
	/// playground tables.
	#[allow(clippy::missing_errors_doc)]
	pub fn render_mut(&self, ctx: &mut RenderContext) -> Result<()> {
		validate_version(ctx)?;

		let config = Config::new(ctx);

		let mut chapters_with_codeblocks = Vec::new();
		let mut result: Result<()> = Ok(());

		ctx.book.for_each_mut(|item| {
			if result.is_err() {
				return;
			}

			if let BookItem::Chapter(chapter) = item {
				debug!("Processing chapter {}", &chapter.name);
				match process_markdown(&config, chapter) {
					Ok(processed) => {
						debug!("Processed chapter {}", &chapter.name);
						if let Some(processed) = processed {
							chapters_with_codeblocks.push(processed);
						}
					}
					Err(error) => result = Err(error),
				};
			}
		});

		debug!("Processed chapters");

		HtmlHandlebars::new().render(ctx)?;

		debug!("Finished rendering");

		if !chapters_with_codeblocks.is_empty() {
			build(ctx, &config, chapters_with_codeblocks)?;
		}

		debug!("Finished");

		Ok(())
	}
}

impl Default for AngularRenderer {
	fn default() -> Self {
		Self::new()
	}
}
