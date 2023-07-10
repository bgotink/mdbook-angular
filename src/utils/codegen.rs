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
