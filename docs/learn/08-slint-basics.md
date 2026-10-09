# Learn 08: Slint basics (phase W0)

Official docs: https://docs.slint.dev (use the version matching `Cargo.toml`, 1.18).

## 1. The `.slint` language (≈ XAML, but reactive)

```slint
export component AppWindow inherits Window {
    in property <string> bs-date;            // set from Rust
    in-out property <bool> nepali;
    callback next-month();                   // handled in Rust

    VerticalLayout {
        Text { text: root.bs-date; font-size: 20px; }
        Button { text: root.nepali ? "अर्को" : "Next"; clicked => { root.next-month(); } }
    }
}
```

- Properties are **reactive bindings**: `text: root.bs-date` updates automatically when
  Rust sets `bs-date`. No `INotifyPropertyChanged`.
- `in` = settable from Rust, `out` = readable from Rust, `in-out` = both.
- Names use `kebab-case` in `.slint`; the generated Rust uses `snake_case`
  (`set_bs_date`, `on_next_month`).

## 2. How `.slint` becomes Rust (`build.rs`)

`build.rs` runs **before** compiling the crate. `slint_build::compile("ui/app.slint")`
translates the markup into Rust code, and `slint::include_modules!()` in `main.rs` pastes it
in. So a typo in `.slint` is a compile error with a file and line.

## 3. Rust side

```rust
slint::include_modules!();
fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    ui.set_bs_date("Ashoj 8, 2083".into());
    let weak = ui.as_weak();                         // weak handle: avoids a reference cycle
    ui.on_next_month(move || {
        if let Some(ui) = weak.upgrade() { ui.set_bs_date("Kartik 1, 2083".into()); }
    });
    ui.run()                                         // blocks: runs the event loop
}
```

## 4. Live preview

Install the "Slint" VS Code extension. Opening a `.slint` file shows a live preview
while you edit. It's the fastest way to learn layouts.
