# Learn 09: Event loop, timers, shared state (phase W1)

## 1. One UI thread

Like WPF's dispatcher: all UI objects live on the main thread, and `ui.run()` loops
over OS events and calls your callbacks. A long computation inside a callback freezes the UI.

## 2. Timers

```rust
let timer = slint::Timer::default();
timer.start(slint::TimerMode::SingleShot, next_wakeup, move || { /* update, re-arm */ });
```

We use a single-shot timer re-armed for the **next minute boundary**, instead of a
repeating 1-second timer, so the process sleeps ~60 s at a time (≈ 0 % idle CPU).

## 3. `Rc<RefCell<T>>`: shared, mutable state on one thread

Rust forbids two mutable references to the same data at compile time. Closures
(callbacks) each need access to `AppState`, so:

- `Rc<T>` = reference-counted shared ownership (single-thread; `Arc` is the thread-safe one).
- `RefCell<T>` moves the "one writer at a time" check to runtime: `.borrow()` /
  `.borrow_mut()`. Two overlapping `borrow_mut()` calls **panic**, so keep borrows
  short and never hold one while calling into Slint (which may call another callback).

```rust
let state = Rc::new(RefCell::new(AppState::default()));
let s = Rc::clone(&state);
ui.on_next_month(move || { s.borrow_mut().go_next_month(); /* borrow ends here */ });
```

## 4. Talking to the UI from another thread

The single-instance listener runs on a background thread. It must not touch UI objects.
It calls `slint::invoke_from_event_loop(move || { ... })`, which queues the closure to run on the
UI thread (≈ `Dispatcher.BeginInvoke`).

## 5. Weak handles

`ui.as_weak()` inside callbacks avoids a cycle (UI owns callback → callback owns UI →
nothing is ever freed). `weak.upgrade()` returns `Option`, `None` if the window is gone.
