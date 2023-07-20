use std::{
	fs,
	path::{Path, PathBuf},
};

use anyhow::Result;

use crate::codeblock::CodeBlock;

pub(crate) fn generate_angular_code(
	project_root: &Path,
	angular_code_samples: Vec<CodeBlock>,
	experimental_builder: bool,
) -> Result<()> {
	fs::create_dir_all(project_root)?;

	let mut main = "\
			import 'zone.js';\n\
			import {NgZone, ApplicationRef} from '@angular/core';\n\
			import {bootstrapApplication} from '@angular/platform-browser';\n\
			const zone = new NgZone({});\n\
			const providers = [{provide: NgZone, useValue: zone}];\n\
			const applications: Promise<ApplicationRef>[] = [];\n\
			(globalThis as any).mdBookAngular = {zone, applications};\
		"
	.to_owned();

	for (index, file) in angular_code_samples.into_iter().enumerate() {
		fs::write(
			&project_root.join(format!("component_{index}.ts")),
			file.source_to_write,
		)?;

		main.push_str(
			format!(
				"\n\
					import {{{} as CodeBlock_{index}}} from './component_{index}.js';\n\
					applications.push(bootstrapApplication(CodeBlock_{index}, {{providers}}));\n\
				",
				file.class_name,
			)
			.as_str(),
		);
	}

	let main_file_name = if experimental_builder {
		let mut p = PathBuf::from(project_root.file_name().unwrap());
		p.set_extension("ts");
		p
	} else {
		Path::new("main.ts").to_owned()
	};

	fs::write(project_root.join(main_file_name), main)?;

	fs::write(
		project_root.join("index.html"),
		"<!doctype html>\n<html></html>\n",
	)?;
	fs::write(
		project_root.join("tsconfig.json"),
		r#"{"extends":"../tsconfig.json","files": ["main.ts"]}"#,
	)?;

	Ok(())
}

pub(crate) fn generated_rendered_code_block(
	code_block: &CodeBlock,
	index: usize,
	add_playground: bool,
	has_playgrounds: &mut bool,
) -> String {
	let mut element = format!("<{0}></{0}>\n", &code_block.tag);

	if !add_playground {
		return element;
	}

	if !code_block.inputs.is_empty() {
		*has_playgrounds = true;

		let inputs = code_block
			.inputs
			.iter()
			.map(|input| {
				format!(
					"\
						<tr>\
							<td>\
								<code>{}</code>\
							</td>\
							<td>{}</td>\
							<td>\
								<mdbook-angular-input name=\"{0}\" index=\"{}\">{}</mdbook-angular-input>\
							</td>\
						</tr>\
					",
					&input.name,
					input.description.as_deref().unwrap_or(""),
					index,
					serde_json::to_string(&input.config)
						.unwrap()
						.replace('<', "&lt;")
				)
			})
			.collect::<String>();

		element.push_str(&format!(
			"\n\
				Inputs:\n\n\
				<table class=\"mdbook-angular mdbook-angular-inputs\">\
					<thead>\
						<th>Name</th>
						<th>Description</th>
						<th>Value</th>
					</thead>\
					<tbody>{inputs}</tbody>\
				</table>\n\n\
			"
		));
	}

	if !code_block.actions.is_empty() {
		*has_playgrounds = true;

		let actions = code_block
			.actions
			.iter()
			.map(|action| {
				format!(
					"\
						<tr>\
							<td>\
								<mdbook-angular-action name=\"{}\" index=\"{}\"></mdbook-angular-action>\
							</td>\
							<td>{}</td>\
						</tr>\
					",
					&action.name, index, action.description,
				)
			})
			.collect::<String>();

		element.push_str(&format!(
			"\n\
				Actions:\n\n\
				<table class=\"mdbook-angular mdbook-angular-actions\">\
					<thead>\
						<th>Action</th>
						<th>Description</th>
					</thead>\
					<tbody>{actions}</tbody>\
				</table>\n\n\
			"
		));
	}

	element
}
