use std::{fs, path::Path};

use anyhow::Result;

use crate::codeblock::CodeBlock;

pub(crate) fn generate_angular_code(
	project_root: &Path,
	angular_code_samples: Vec<CodeBlock>,
) -> Result<()> {
	fs::create_dir(project_root)?;

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
			file.source,
		)?;

		main.push_str(
        format!(
					"\nimport {{{} as CodeBlock_{index}}} from './component_{index}';\napplications.push(bootstrapApplication(CodeBlock_{index}, {{providers}}));\n",
					file.class_name,
				).as_str()
			);
	}

	fs::write(Path::join(project_root, "main.ts"), main)?;

	fs::write(
		Path::join(project_root, "index.html"),
		"<!doctype html>\n<html></html>\n",
	)?;
	fs::write(
		Path::join(project_root, "tsconfig.json"),
		"{\"extends\":\"../tsconfig.json\",\"files\": [\"main.ts\"]}",
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

	if add_playground && !code_block.inputs.is_empty() {
		*has_playgrounds = true;

		let inputs = code_block
									.inputs
									.iter()
									.map(|input| {
										format!(
											"<tr><td><code class=\"hljs\">{}</code></td><td>{}</td><td><mdbook-angular-input name=\"{0}\" index=\"{}\">{}</mdbook-angular-input></td></tr>",
											&input.name,
											input
												.description
												.as_deref()
												.unwrap_or(""),
											index,
											serde_json::to_string(&input.config).unwrap().replace('<', "&lt;")
										)
									})
									.collect::<String>();

		element.push_str(&format!(
			"\n\
				Inputs:\n\n\
				<table>\
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

	element
}
