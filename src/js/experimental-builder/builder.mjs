// If you change this file, run `yarn build-scripts`

import {createBuilder} from '@angular-devkit/architect';
import fs from 'node:fs';
import {posix} from 'node:path';

/**
 * @returns {Promise<import('@angular-devkit/build-angular/node_modules/@angular/build/src/builders/application/index.js')>}
 */
async function getBuildModule() {
	try {
		return await import(
			// @ts-expect-error This file no longer exists in Angular 18
			'@angular-devkit/build-angular/src/builders/application/index.js'
		);
	} catch {}

	try {
		// @ts-expect-error This file only exists if @angular/build is installed
		return await import('@angular/build/private');
	} catch {}
	try {
		return await import(
			'@angular-devkit/build-angular/node_modules/@angular/build/src/private.js'
		);
	} catch {}

	throw new Error(
		'Unable to find builder function in @angular-devkit/build-angular or @angular/build',
	);
}

export default createBuilder(async function* (
	{optimization, watch = false, ...input},
	context,
) {
	yield* (await getBuildModule()).buildApplicationInternal(
		{
			watch,
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
