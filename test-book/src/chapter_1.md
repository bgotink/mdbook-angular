# Chapter 1

This is a test of the alarm system

```ts,angular,hide
import {Component, ChangeDetectionStrategy, Input} from '@angular/core';

@Component({
  selector: 'codeblock-0',
  standalone: true,
  template: `<p>Hi {{name}}, it's working!</p>`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CodeBlock {
	/**
	 * Person to tell it's working
	 *
	 * @input {"default": "Bram"}
	 */
	@Input()
	name = 'Bram';
}
```

more test stuff

```ts,angular
import {Component, ChangeDetectionStrategy} from '@angular/core';

@Component({
  standalone: true,
  template: `<p>It's working well!</p>`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CodeBlock {}
```

even more test stuff

```ts,angular
import {Component, ChangeDetectionStrategy, signal} from '@angular/core';

@Component({
  selector: 'my-test',
  standalone: true,
  template: `<p>{{counter()}} <button (click)="increase()">increase</button></p>`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TestComponent {
  /** @keep lalalal */
  counter = signal(0);

  /**
   * @keep
   * lalalalalala
   */
  increase() {
    this.counter.set(this.counter() + 1);
  }
}
```
