// If you change this file, run `yarn build-scripts`

/** @type {typeof document.createElement} */
let create = name => document.createElement(name);
let attr = (self, name) => self.getAttribute(name);
let on = (element, name, listener) => element.addEventListener(name, listener);

customElements.define(
	'mdbook-angular-input',
	class MdbookAngularInputElement extends HTMLElement {
		#processed = false;

		connectedCallback() {
			if (this.#processed) {
				return;
			}
			this.#processed = true;

			const config = JSON.parse(this.innerText);

			let input;
			let getValue;

			if (typeof config.type === 'object' && 'enum' in config.type) {
				input = create('select');
				input.append(
					...config.type.enum.map(value => {
						const option = create('option');
						option.value = value;
						option.innerText = value;
						option.checked = value === config.default;
						return option;
					}),
				);

				getValue = () => input.value;
			} else {
				switch (config.type) {
					case 'number': {
						input = create('input');
						input.type = 'number';
						input.valueAsNumber = config.default;

						getValue = () => input.valueAsNumber;
						break;
					}
					case 'boolean': {
						input = create('input');
						input.type = 'checkbox';
						input.checked = config.default;

						getValue = () => input.checked;
						break;
					}
					default: {
						input = create('input');
						input.type = 'text';
						input.value = config.default || '';

						getValue = () => input.value;
						break;
					}
				}
			}

			while (this.firstChild) {
				this.firstChild.remove();
			}

			this.append(input);

			const name = attr(self, 'name');
			const index = +attr(self, 'index');

			function update() {
				let app =
					/** @type {Promise<import('@angular/core').ApplicationRef>} */ (
						mdBookAngular.applications[index]
					);
				let zone = /** @type {import('@angular/core').NgZone} */ (
					mdBookAngular.zone
				);

				app.then(app => {
					const component = app.components[0];

					zone.run(() => {
						component.setInput(name, getValue());
					});
				});
			}

			let throttleTimeout = null;

			function throttledUpdate() {
				if (throttleTimeout != null) {
					clearTimeout(throttleTimeout);
				}

				throttleTimeout = setTimeout(update, 300);
			}

			on(input, 'change', update);
			on(input, 'input', throttledUpdate);
		}
	},
);

customElements.define(
	'mdbook-angular-action',
	class MdbookAngularActionElement extends HTMLElement {
		#processed = false;

		connectedCallback() {
			if (this.#processed) {
				return;
			}
			this.#processed = true;

			while (this.firstChild) {
				this.firstChild.remove();
			}

			const name = attr(self, 'name');
			const index = +attr(self, 'index');

			const button = create('button');
			const code = create('code');
			code.append(`${name}()`);
			button.append(code);
			this.append(button);

			on(button, 'click', () => {
				let app =
					/** @type {Promise<import('@angular/core').ApplicationRef>} */ (
						mdBookAngular.applications[index]
					);
				let zone = /** @type {import('@angular/core').NgZone} */ (
					mdBookAngular.zone
				);

				app.then(app => {
					const component = app.components[0];

					zone.run(() => {
						component.instance[name]();
					});
				});
			});
		}
	},
);
