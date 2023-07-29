//! mdbook preprocessor that compiles and renders angular code samples

use std::{
	env,
	io::{self, Write},
};

use anyhow::Result;
use log::{warn, LevelFilter};
use mdbook::renderer::RenderContext;
use mdbook_angular::{stop_background_process, AngularRenderer, Config};

fn main() -> Result<()> {
	init_logger();

	let mut args = env::args();
	let _ = args.next();

	if let Some(arg) = args.next() {
		if arg == "stop" {
			let config = Config::read(env::current_dir()?)?;

			return stop_background_process(&config);
		}

		warn!("Unexpected command {arg}");
	}

	let mut ctx = RenderContext::from_json(io::stdin())?;

	AngularRenderer::new().render_mut(&mut ctx)
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
		builder.filter(None, LevelFilter::Info);
		// Filter extraneous html5ever not-implemented messages
		builder.filter(Some("html5ever"), LevelFilter::Error);
	}

	builder.init();
}
