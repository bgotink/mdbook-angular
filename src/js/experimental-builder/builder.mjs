import {buildApplicationInternal} from '@angular-devkit/build-angular/src/builders/application/index.js';
import {createBuilder} from '@angular-devkit/architect';
import {existsSync, readdirSync, statSync} from 'node:fs';
import {posix} from 'node:path';

export default createBuilder((input, context) => {
	return buildApplicationInternal(
		{
			watch: false,
			progress: false,

			index: false,
			entryPoints: new Set(
				readdirSync(context.workspaceRoot)
					.filter(file => {
						let resolvedFile = posix.join(context.workspaceRoot, file);

						return (
							statSync(resolvedFile).isDirectory() &&
							existsSync(`${resolvedFile}/${file}.ts`)
						);
					})
					.map(file => `${file}/${file}.ts`),
			),
			aot: true,

			tsConfig: 'tsconfig.json',
			outputPath: input.outputPath,
			inlineStyleLanguage: input.inlineStyleLanguage,

			...(input.optimization
				? {
						optimization: {
							styles: {
								inlineCritical: false,
								minify: true,
							},
							scripts: true,
						},
						outputHashing: 'all',
				  }
				: {
						optimization: false,
						outputHashing: 'none',
				  }),
		},
		context,
	);
});
