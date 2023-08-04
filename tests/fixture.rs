use std::{
	collections::HashMap,
	fs,
	path::Path,
	process::{Command, Stdio},
};

use anyhow::Result;
use assert_cmd::cargo::cargo_bin;
use copy_dir::copy_dir;
use select::{document::Document, predicate::*};
use tempfile::TempDir;

pub struct Fixture(TempDir);

#[inline]
fn symlink_dir<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<()> {
	#[cfg(windows)]
	std::os::windows::fs::symlink_dir(from, to)?;
	#[cfg(not(windows))]
	std::os::unix::fs::symlink(from, to)?;
	Ok(())
}

impl Fixture {
	pub fn run_without_build(env: Option<HashMap<String, String>>) -> Fixture {
		let mut map = env.unwrap_or_default();
		map.insert("MDBOOK_ANGULAR_SKIP_BUILD".to_owned(), "1".to_owned());
		Self::run(Some(map))
	}

	pub fn run(env: Option<HashMap<String, String>>) -> Fixture {
		let temp_dir = tempfile::Builder::new()
			.prefix("mdbook-angular-tests")
			.tempdir()
			.expect("failed to create temporary directory");

		let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture");

		symlink_dir(
			Path::new(env!("CARGO_MANIFEST_DIR")).join("node_modules"),
			temp_dir.path().join("node_modules"),
		)
		.expect("Failed to link node_modules");

		copy_dir(fixture_dir.join("src"), temp_dir.path().join("src"))
			.expect("Failed to copy source");
		fs::copy(
			fixture_dir.join("book.toml"),
			temp_dir.path().join("book.toml"),
		)
		.expect("Failed to copy book.toml");

		let mut command = Command::new("yarn");
		command
			.arg("exec")
			.arg("mdbook")
			.arg("build")
			.env_remove("RUST_LOG")
			.env(
				"MDBOOK_OUTPUT__ANGULAR__COMMAND",
				cargo_bin("mdbook-angular"),
			)
			.current_dir(&temp_dir)
			.stdin(Stdio::null())
			.stdout(Stdio::null())
			.stderr(Stdio::inherit());

		if let Some(env) = env {
			for (name, val) in env {
				command.env(name, val);
			}
		}

		let result = command.status().expect("failed to spawn mdbook");
		if !result.success() {
			panic!("mdbook build failed");
		}

		Fixture(temp_dir)
	}

	fn read_chapter(&self, path: &str) -> Result<Chapter> {
		let html = fs::read_to_string(self.0.path().join("book").join(path))?;

		Ok(Chapter(Document::from(html.as_str())))
	}

	pub fn chapter1(&self) -> Chapter {
		self.read_chapter("chapter-1.html")
			.expect("couldn't read chapter-1.html")
	}

	pub fn chapter2(&self) -> Chapter {
		self.read_chapter("chapter-2.html")
			.expect("couldn't read chapter-2.html")
	}

	pub fn chapter3(&self) -> Chapter {
		self.read_chapter("chapter-3.html")
			.expect("couldn't read chapter-3.html")
	}

	pub fn chapter4(&self) -> Chapter {
		self.read_chapter("chapter-4.html")
			.expect("couldn't read chapter-4.html")
	}

	pub fn chapter5(&self) -> Chapter {
		self.read_chapter("chapter-5.html")
			.expect("couldn't read chapter-5.html")
	}
}

pub struct Chapter(Document);

impl Chapter {
	pub fn assert_code_block_count(&self, count: usize) {
		assert_eq!(count, self.0.find(Name("pre").child(Name("code"))).count());
	}

	pub fn assert_collapsed(&self, value: bool) {
		for code in self.0.find(Name("pre").child(Name("code"))) {
			assert_eq!(
				value,
				code.parent().unwrap().parent().unwrap().is(Name("details"))
			);
		}
	}

	pub fn assert_is_default_insertion(&self, value: bool) {
		assert_eq!(1, self.0.find(Name("example-inline")).count());
		assert_eq!(
			!value,
			self.0
				.find(Name("example-inline"))
				.next()
				.unwrap()
				.is(Name("blockquote").descendant(Name("example-inline")))
		);

		assert_eq!(1, self.0.find(Name("example-component")).count());
		assert_eq!(
			!value,
			self.0
				.find(Name("example-component"))
				.next()
				.unwrap()
				.is(Name("blockquote").descendant(Name("example-component")))
		);
	}

	pub fn assert_has_playground(&self, has_playground: bool) {
		if !has_playground {
			assert_eq!(0, self.0.find(Class("mdbook-angular-inputs")).count());
			return;
		}

		assert_eq!(2, self.0.find(Class("mdbook-angular-inputs")).count());
		assert_eq!(
			2,
			self.0
				.find(Class("mdbook-angular-inputs").descendant(Name("mdbook-angular-input")))
				.count()
		);

		for (index, input) in self
			.0
			.find(Class("mdbook-angular-inputs").descendant(Name("mdbook-angular-input")))
			.enumerate()
		{
			assert_eq!(Some(index.to_string().as_str()), input.attr("index"));
			assert_eq!(
				r#"{"type":"string","default":"lorem ipsum"}"#,
				input.inner_html()
			);
		}
	}
}
