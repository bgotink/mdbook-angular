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
	 */
	@Input()
	name: 'Bram' | 'reader' = 'Bram';
}

@Component({
	standalone: true,
	selector: 'convince-me',
	template: `<p>It's working well, dear {{ name }} {{ exclaim }}</p>`,
	changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ConvinceComponent {
	/**
	 * Person to convince
	 */
	@Input()
	name = 'Bram';

	exclaim = '';

	/**
	 * Number of exclamation points to write!
	 */
	@Input()
	set numberOfExclamationPoints(value: number) {
		this.exclaim = '!'.repeat(value);
	}
}
