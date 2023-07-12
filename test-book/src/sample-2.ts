import {Component, ChangeDetectionStrategy, Input} from '@angular/core';

@Component({
	selector: 'announce-it',
	standalone: true,
	template: `<p>Hi {{ name }}, it's working!</p>`,
	changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AnnounceComponent {
	/**
	 * Person to tell it's working
	 *
	 * @input {"default": "Bram"}
	 */
	@Input()
	name = 'Bram';
}

@Component({
	standalone: true,
	selector: 'convince-me',
	template: `<p>It's working well, dear {{ name }} !</p>`,
	changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ConvinceComponent {
	/**
	 * Person to convince
	 * @input {"default": "Bram"}
	 */
	@Input()
	name = 'Bram';
}
