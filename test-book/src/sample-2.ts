import {Component, ChangeDetectionStrategy, Input} from '@angular/core';

@Component({
	standalone: true,
	template: `<p>It's working well, dear {{ name }} !</p>`,
	changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CodeBlock {
	/**
	 * Person to convince
	 * @input {"default": "Bram"}
	 */
	@Input()
	name = 'Bram';
}
