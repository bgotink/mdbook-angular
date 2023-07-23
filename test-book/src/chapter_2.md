# Chapter 2

This is a test of the alarm system

even more test stuff

```ts,angular,collapsed
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

	/**
	 * Reset the counter
	 * @action
	 */
	reset() {
		this.counter.set(0);
	}
}
```

## Section

Lorem ipsum dolor sit amet, <test-exclaim></test-exclaim>

```ts,angular,no-insert
import {Component, ChangeDetectionStrategy, Input} from '@angular/core';

@Component({
  selector: 'test-exclaim',
  standalone: true,
  template: `{{name}}!`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TestComponent {
  @Input() name = 'Bram';
}
```
