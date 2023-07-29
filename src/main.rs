//! mdbook preprocessor that compiles and renders angular code samples

mod angular;
pub(crate) mod codeblock;
pub(crate) mod config;
mod js;
mod markdown;
mod utils;

use std::{
	env,
	io::{self, Write},
};

use angular::build;
use anyhow::Result;
use config::Config;
use log::warn;
use markdown::process_markdown;
use mdbook::{
	renderer::{HtmlHandlebars, RenderContext},
	BookItem, Renderer,
};

fn main() -> Result<()> {
	init_logger();

	let mut args = env::args();
	let _ = args.next();

	if let Some(arg) = args.next() {
		if arg == "stop" {
			return stop_background_process();
		}

		warn!("Unexpected command {arg}");
	}

	render()
}

#[cfg(all(unix, feature = "background"))]
fn stop_background_process() -> Result<()> {
	use angular::stop_background_process;

	let config = Config::read(env::current_dir()?)?;

	stop_background_process(&config)
}

#[cfg(not(all(unix, feature = "background")))]
fn stop_background_process() -> Result<()> {
	use anyhow::Error;

	Err(Error::msg("Stop command is not supported"));
}

fn render() -> Result<()> {
	let mut ctx = RenderContext::from_json(io::stdin())?;
	let config = Config::new(&ctx);

	let mut chapters_with_codeblocks = Vec::new();
	let mut result: Result<()> = Ok(());

	ctx.book.for_each_mut(|item| {
		if result.is_err() {
			return;
		}

		if let BookItem::Chapter(chapter) = item {
			log::debug!("Processing chapter {}", &chapter.name);
			match process_markdown(&config, chapter) {
				Ok(processed) => {
					log::debug!("Processed chapter {}", &chapter.name);
					if let Some(processed) = processed {
						chapters_with_codeblocks.push(processed);
					}
				}
				Err(error) => result = Err(error),
			};
		}
	});

	log::debug!("Processed chapters");

	result?;

	HtmlHandlebars::new().render(&ctx)?;

	log::debug!("Finished rendering");

	if !chapters_with_codeblocks.is_empty() {
		build(&ctx, &config, chapters_with_codeblocks)?;
	}

	log::debug!("Finished");

	Ok(())
}

fn init_logger() {
	let mut builder = env_logger::Builder::new();

	builder.format(|formatter, record| {
		writeln!(
			formatter,
			"{} [{}] ({}): {}",
			chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
			record.level(),
			record.target(),
			record.args()
		)
	});

	if let Ok(var) = env::var("RUST_LOG") {
		builder.parse_filters(&var);
	} else {
		// if no RUST_LOG provided, default to logging at the Info level
		builder.filter(None, log::LevelFilter::Info);
		// Filter extraneous html5ever not-implemented messages
		builder.filter(Some("html5ever"), log::LevelFilter::Error);
	}

	builder.init();
}
