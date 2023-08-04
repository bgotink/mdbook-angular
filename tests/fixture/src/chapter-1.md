# Chapter 1

> no flags

Inline

```ts angular
import {ChangeDetectionStrategy, Component, Input} from '@angular/core';

@Component({
	standalone: true,
	selector: 'example-inline',
	template: `I'm a good inline example`,
	changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExampleComponent {
	@Input()
	text = 'lorem ipsum';
}
```

External

{{#angular ./example.ts#ExampleComponent}}
