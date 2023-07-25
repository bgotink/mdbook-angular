// If you change this file, run `yarn build-scripts`

import {buildApplicationInternal} from '@angular-devkit/build-angular/src/builders/application/index.js';
import {createBuilder} from '@angular-devkit/architect';
import fs from 'node:fs';
import {posix} from 'node:path';

export default createBuilder(({optimization, ...input}, context) => {
	return buildApplicationInternal(
		{
			watch: false,
			progress: false,

			index: false,
			entryPoints: new Set(
				fs
					.readdirSync(context.workspaceRoot)
					.filter(file => {
						let resolvedFile = posix.join(context.workspaceRoot, file);

						return (
							fs.statSync(resolvedFile).isDirectory() &&
							fs.existsSync(`${resolvedFile}/${file}.ts`)
						);
					})
					.map(file => `${file}/${file}.ts`),
			),
			aot: true,

			tsConfig: 'tsconfig.json',
			...input,

			...(optimization
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
