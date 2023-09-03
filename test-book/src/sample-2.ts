import {
	Component,
	ChangeDetectionStrategy,
	Input,
	ENVIRONMENT_INITIALIZER,
	Provider,
} from '@angular/core';

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
	name: 'Bram' | 'reader' = 'reader';

	@Input()
	notUsed = -1;

	@Input()
	notUsedEither = 10 * 1024 * 1024;
}

@Component({
	standalone: true,
	selector: 'convince-me',
	template: `<p>It's working well, dear {{ name }} {{ exclaim }}</p>`,
	changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ConvinceComponent {
	static rootProviders: Provider[] = [
		{
			provide: ENVIRONMENT_INITIALIZER,
			multi: true,
			useValue: () => {
				console.log('provided');
			},
		},
	];

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
