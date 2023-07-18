//! mdbook preprocessor that compiles and renders angular code samples

mod codeblock;
mod codeblock_collector;
mod utils;
mod worker;

use std::{
	env,
	io::{self, Write},
};

use chrono::Local;

use crate::worker::AngularWorker;
use env_logger::Builder;
use log::LevelFilter;
use mdbook::{
	errors::Error,
	renderer::{HtmlHandlebars, RenderContext},
	BookItem, Renderer,
};

fn main() -> Result<(), Error> {
	init_logger();

	let mut ctx = RenderContext::from_json(io::stdin())?;

	let valid_mdbook_versions = semver::VersionReq::parse(mdbook::MDBOOK_VERSION)?;
	let actual_mdbook_version = semver::Version::parse(&ctx.version)?;

	if !valid_mdbook_versions.matches(&actual_mdbook_version) {
		return Err(Error::msg(format!(
			"Expected mdbook version {valid_mdbook_versions} but got {actual_mdbook_version}"
		)));
	}

	let renderer = HtmlHandlebars::new();
	let worker = prepare(&mut ctx)?;

	if let Some(value) = ctx.config.get("output.angular.html") {
		ctx.config.set("output.html", value.clone())?;
	}

	renderer.render(&ctx)?;

	worker.finalize()?;

	Ok(())
}

fn init_logger() {
	let mut builder = Builder::new();

	builder.format(|formatter, record| {
		writeln!(
			formatter,
			"{} [{}] ({}): {}",
			Local::now().format("%Y-%m-%d %H:%M:%S"),
			record.level(),
			record.target(),
			record.args()
		)
	});

	if let Ok(var) = env::var("RUST_LOG") {
		builder.parse_filters(&var);
	} else {
		// if no RUST_LOG provided, default to logging at the Info level
		builder.filter(None, LevelFilter::Info);
		// Filter extraneous html5ever not-implemented messages
		builder.filter(Some("html5ever"), LevelFilter::Error);
	}

	builder.init();
}

fn prepare(ctx: &mut RenderContext) -> Result<AngularWorker, Error> {
	let mut worker = AngularWorker::new(ctx)?;

	let mut errors: Vec<Error> = Vec::new();

	ctx.book.for_each_mut(|item| {
		if let BookItem::Chapter(ref mut chapter) = item {
			log::debug!("Processing chapter '{}'", chapter.name);
			if let Err(err) = worker.process_chapter(chapter) {
				errors.push(err.context(format!("in chapter {}", chapter.name)));
			}
		}
	});

	if let Some(error) = errors.into_iter().next() {
		return Err(error);
	}

	Ok(worker)
}
