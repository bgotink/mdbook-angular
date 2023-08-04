# Chapter 2

> no-insert  
> <example-inline></example-inline>  
> <example-component></example-component>

Inline

```ts angular no-insert
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

{{#angular ./example.ts#ExampleComponent no-insert}}
