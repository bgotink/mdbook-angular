#![allow(clippy::panic, clippy::expect_used)]

use std::{
	fs::File,
	process::{Command, Stdio},
};

pub fn main() {
	println!("cargo:rerun-if-changed=yarn.lock");

	build("src/js/playground-io.js", "src/js/playground-io.min.js");
	build(
		"src/js/experimental-builder/builder.mjs",
		"src/js/experimental-builder/builder.min.mjs",
	);
}

fn build(input: &str, output: &str) {
	println!("cargo:rerun-if-changed={input}");

	let input = File::open(input).expect("Failed to open input file ");
	let output = File::create(output).expect("Failed to open output file");

	let result = Command::new("yarn")
		.arg("esbuild")
		.arg("--minify")
		.arg("--format=esm")
		.arg("--loader=js")
		.stdout(output)
		.stderr(Stdio::inherit())
		.stdin(input)
		.status()
		.expect("Failed to start esbuild");

	if result.success() {
		return;
	}

	if let Some(exit_code) = result.code() {
		panic!("ESBuild exited with code {exit_code}");
	} else {
		panic!("ESBuild failed");
	}
}
