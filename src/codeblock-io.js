customElements.define(
	'mdbook-angular-input',
	class MdbookAngularInputElement extends HTMLElement {
		processed = false;

		connectedCallback() {
			if (this.processed) {
				return;
			}
			this.processed = true;

			const config = JSON.parse(this.innerText);

			let input;
			let getValue;

			if (typeof config.type === 'object' && 'enum' in config.type) {
				input = document.createElement('select');
				input.append(
					...config.type.enum.map(value => {
						const option = document.createElement('option');
						option.value = value;
						option.innerText = value;
						option.checked = value === config.default;
						return option;
					}),
				);

				getValue = () => input.value;
			} else {
				switch (config.type) {
					case 'Number': {
						input = document.createElement('input');
						input.type = 'number';
						input.valueAsNumber = config.default;

						getValue = () => input.valueAsNumber;
						break;
					}
					case 'Boolean': {
						input = document.createElement('input');
						input.type = 'checkbox';
						input.checked = config.default;

						getValue = () => input.checked;
						break;
					}
					default: {
						input = document.createElement('input');
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

			const name = this.getAttribute('name');
			const index = +this.getAttribute('index');

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

			input.addEventListener('change', update);
			input.addEventListener('input', throttledUpdate);
		}
	},
);
