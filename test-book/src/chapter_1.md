# Chapter 1

This is a test of the alarm system

```ts,angular
import {Component, ChangeDetectionStrategy} from '@angular/core';

@Component({
  selector: 'codeblock-0',
  standalone: true,
  template: `<p>It's working!</p>`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CodeBlock {}
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
  counter = signal(0);

  increase() {
    this.counter.set(this.counter() + 1);
  }
}
```
