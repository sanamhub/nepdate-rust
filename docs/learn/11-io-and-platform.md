# Learn 11: Files, serde, platform code (phase W3)

## 1. Platform paths

`dirs::config_dir()` / `dirs::data_local_dir()` return the right base folder per OS
(`%APPDATA%`, `~/Library/Application Support`, `$XDG_CONFIG_HOME`). We append
`nepdate-widget/`. Never hard-code paths.

## 2. Atomic writes

Writing `settings.toml` directly can leave a half-written file if the app crashes or
power fails mid-write. Instead:

```rust
let tmp = path.with_extension("toml.tmp");
std::fs::write(&tmp, text)?;       // 1. write everything to a temp file in the SAME folder
std::fs::rename(&tmp, path)?;      // 2. rename over the old file (atomic on the same volume)
```

## 3. serde defaults

```rust
#[derive(Serialize, Deserialize)]
#[serde(default)]                   // any missing key → value from Default::default()
pub struct Settings { pub language: String, pub clock_24h: bool, /* ... */ }
```

Unknown keys are ignored unless you add `#[serde(deny_unknown_fields)]` (we don't, so old
and new versions can share a settings file).

## 4. Platform-specific code

```rust
#[cfg(target_os = "windows")]
fn hide_from_taskbar(w: &slint::Window) { /* ... */ }
#[cfg(not(target_os = "windows"))]
fn hide_from_taskbar(_w: &slint::Window) {}
```

Keep these tiny and inside `platform/`. The CI matrix compiles each branch on its own OS.

## 5. Threads for blocking work

The single-instance listener blocks waiting for connections, so it runs on
`std::thread::spawn`. Data sent between threads must be `Send`. The compiler enforces
this, so a data race is a compile error rather than a production bug.
