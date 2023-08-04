import {ChangeDetectionStrategy, Component, Input} from '@angular/core';

@Component({
	standalone: true,
	selector: 'example-component',
	template: `I'm a good example`,
	changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExampleComponent {
	@Input()
	text = 'lorem ipsum';
}
