# mdbook-angular

A renderer for [mdbook](https://rust-lang.github.io/mdBook/index.html) that turns angular code samples into running angular applications.

## Usage

Install the `mdbook-angular` binary in your `PATH` and enable the renderer in your `book.toml` by adding an `output.angular` section.

There are two ways to include live code samples in your book.
The first option is to add `angular` to a typescript or javascript code block, e.g.

````markdown
```ts,angular
// live angular sample here
```
````

The second option is to add a special `{{#angular}}` tag in your page that points towards a component

```markdown
{{#angular ./path/to/file.ts#NameOfExportedComponentClass}}

<!-- or -->

{{#angular ./path/to/file.ts}}
```

### Inline code blocks

When using a code block with ` ```ts,angular`, the code block has to contain a single exported class that's a valid standalone component.
If the component doesn't define a selector, one will be added.

The code block will be written to a typescript (or javascript) file inside the working directory of the plugin,
so any relative imports in the code block will not work correctly.

Flags can be added to the code block in the language tag, e.g. ` ```ts,angular,hide,playground`.

### The `{{#angular}}` tag

The `{{#angular}}` tag comes in two flavours: you either point towards a file, or specifically towards an export in a file.

The format of the tag is

```
{{#angular <file>[#<exportName>][ flag]*}}
```

Some examples:

```
{{#angular ./example.ts}}
{{#angular ./example.ts hide playground}}
{{#angular ./example.ts#ExampleOneComponent no-playground}}
```

If an export name is present in the tag, the file should export a standalone component with that name.
This component must have a selector, and that selector is expected to be unique on the page.

If no export name is present in the tag, the file should export a single standalone component.
If that component doesn't have a selector, one will be added.

If the `hide` flag is not set, a code block will be added at the location of the `{{#angular}}` tag.
What is shown in the code block depends on whether the name of an exported component was passed or not.
If an export name is passed, only that class and any decorators or surrounding comments will be shown.
If no export name is passed, the entire file will be shown.

### Live examples

All components used in the angular code blocks or imported via the `{{#angular}}` tag will be shown
live on the page.
Every component will be bootstrapped as a _separate_ angular application via [`bootstrapApplication`](https://angular.io/api/platform-browser/bootstrapApplication).

If the `no-insert` flag is not present, the live application will be added below the code block and above the playground.
If the `insert` flag is set, the application will not be added to the page. Instead, you will be responsible for placing the application's element somewhere on the page.
Note angular limits you to a single instance of the component, so placing the element on the page multiple times will not work.

### Playgrounds

Components running as live examples can define inputs and actions.
If a component has at least one input or action, a playground will be added below the live example unless disabled via flag or configuration.

#### Inputs

Inputs are defined via Angular's `@Input()` decorator. The following limitations apply:

- Input renaming is not supported
- The input type must be limited to
  - Text (string)
  - Numbers
  - Booleans
  - Enums with string values

If the input has a default value, the type will be inferred when possible.

You can configure the input's type and default value in the playground by adding an explicit `@input` to a comment above the `@Input()` property. Immediately following the `@input` must be a valid JSON object with key "type" and optional key "default".

The following two properties will both be detected as a "string" input with a default value "Bram".

```ts
// explicit:

/**
 * Author of this document
 * @input {"type": "string", "default": "Bram"}
 */
@Input()
author;

// or inferred

/**
 * Author of this document
 */
@Input()
author = "Bram";
```

You can combine the two methods, e.g. in the following property the type will be set to the enum "morning" or "evening" and the default value will be "evening":

```ts
/**
 * The current time of day
 *
 * @input {"type": {"enum": ["morning", "evening"]}}
 */
@Input()
timeOfDay = "evening";
```

The `"type"` property passed in the `@input` JSON object supports the following values:

- `"string"`
- `"number"`
- `"boolean"`
- an object with a single key `"enum"` pointing towards an array of strings.

#### Actions

Actions are methods on the component class that are annotated with `@action` in a comment block above the method.

```ts
/**
 * Reset the counter
 * @action
 */
reset() {
	this.counter.set(0);
}
```

### Flags

The following flags can be passed on every angular code block:

- `hide`: Don't show the code, but do include the running angular application and possibly the playground
- `playground` / `no-playground`: Show or don't show a playground for the current application, regardless of whether the configuration allows playgrounds. The `playground` flag won't show a playground if the component doesn't warrant a playground.
- `collapsed`: Place the code inside a `<details>` element, hiding it until the user clicks to uncollapse the element.
- `no-insert`: Do not automatically insert the live application on the page. This allows you to write the element linked to the angular component once (and no more than once) on the page at a location of your choosing.

### Configuration

You can configure the following settings:

```toml
[output.angular]

# Option defined by mdbook itself:

# Executable to run, e.g. if mdbook-angular is not on your PATH
command = "/path/to/mdbook-angular"

# Options changing mdbook-angular behaviour:

# Path to a directory used by mdbook-angular as temporary folder,
# relative to the book.toml file
workdir = "mdbook_angular"

# Enable an experimental builder that builds the entire book in a
# single angular build (requires angular ≥ 16.2.0), instead of building
# every chapter separately.
experimentalBuilder = true

# Whether to allow playgrounds, i.e. to add inputs and actions to the page
# allowing your readers to interact with the running code blocks.
#
# This can be overridden per code block by adding either the playground or
# no-playground flag
playgrounds = true

# Options related to the angular build:

# Path to a tsconfig file to use for the build, relative to the book.toml file.
tsconfig = # empty by default

# Language to use for inline styles
inlineStyleLanguage = "css"

# Whether to create an optimized angular build or not
optimize = false
```

None of these settings are required, the default values are shown in the code above.

## Development

This project requires mdbook and angular to be installed

```shell
yarn install
cargo install mdbook
```

Build the project

```shell
cargo build
```

Then run the following command inside the `test-book` folder

```shell
yarn exec mdbook serve

# if you've got @angular/cli installed globally, you can also run
mdbook serve
# directly
```

and point your browser towards `http://localhost:3000`

## License

This project is licensed under the European Union Public License v. 1.2 or later. The full license text can be found in `LICENSE.md`, on [the SPDX website](https://spdx.org/licenses/EUPL-1.2.html), or in any EU member language at [the website of the European Commission](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12).
