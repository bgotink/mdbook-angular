# mdbook-angular

A renderer for [mdbook](https://rust-lang.github.io/mdBook/index.html) that turns angular code samples into running angular applications.

**This project is very experimental and hacked together**

## Development

This project requires mdbook and angular to be installed

```shell
cargo install mdbook
cd test-book && yarn install
```

Build the project

```shell
cargo build
```

Then run the following command inside the `test-book` folder

```shell
mdbook serve
```

and point your browser towards `http://localhost:3000`

## License

This project is licensed under the European Union Public License v. 1.2 or later. The full license text can be found in `LICENSE.md`, on [the SPDX website](https://spdx.org/licenses/EUPL-1.2.html), or in any EU member language at [the website of the European Commission](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12).
