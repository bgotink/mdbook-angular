# Changelog

## Unreleased

<!-- add new items here -->

## v0.4.0

- Drop support for `@angular-devkit/build-angular`, require `@angular/build` to be installed as top-level package
- Remove the old "slow" builder
- Replace the fast "experimental" builder with a new implementation that doesn't use any private Angular APIs

## v0.3.0

- Move from Angular 16 to Angular 17. Due to Angular adding `/browser` to the output folder of the application builder, we can't support both versions simultaneously.
- Support simple binary expressions in default values, making it possible to e.g. write 10 MiB as `10 * 1024 * 1024` instead of `10485760`

## v0.2.1

- Support setters with an `@Input()` decorator
- Decrease the need for explicit `@input` configuration in comment:
  - Use explicit type on input properties when detecting input type
  - Parse (some) unary expressions that yield a known type
  - Map string union types into enumerations
- Fix default property in enumeration always being set to the first option
- Fix negative numbers not being detected as default value
- Make it possible for code blocks to add root-level providers

## v0.2.0

- Allow extra flags passed via `{{#angular}}` block, just like in ` ```ts,angular ` code blocks
- Improve (some) error messages to help debug issues
- Replace multiple builder flags with single builder enum

## v0.1.5

- Add `collapsed` option in `book.toml` to collapse code blocks by default
- Add `uncollapsed` flag for code blocks to overrule the `collapsed` option
- Ensure the "Inputs:" text is in a separate paragraph from the playground
- Disable `@angular/cli` analytics

## v0.1.4

- Fail when invalid configuration is passed

## v0.1.3

- Ensure build succeeds without "background" feature

## v0.1.2

This release is purely to fix the workflows that generate the pre-built binarires

## v0.1.1

- Add `polyfills` option to `book.toml` to configure polyfills
- Fix playground script referring to `this` as `self`

## v0.1.0

- Add experimental background option for \*nix platforms
- Replace camelCase options with kebab-case in `book.toml`
- This package is now also a library, though that's mostly for internal
  organizing and not really because it would be useful to import.

## v0.0.3

- Publish built binaries on GitHub

## v0.0.2

- Fix the `hide` flag
- Use correct "string" input type in README rather than the mistaken "text"

## v0.0.1

Initial release
