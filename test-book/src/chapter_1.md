# Chapter 1

This is a test of the alarm system

{{#angular ./sample-2.ts#AnnounceComponent hide}}

more test stuff

{{#angular ./sample-2.ts#ConvinceComponent}}

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
