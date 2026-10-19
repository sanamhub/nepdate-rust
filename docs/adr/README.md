# Architecture Decision Records

Immutable once accepted (ADR-0001). Supersede with a new ADR.

| ADR | Title | Status |
|---|---|---|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-cargo-workspace-monorepo.md) | One Rust repository and Cargo workspace for library and widget | Accepted |
| [0003](0003-shared-data-and-spec.md) | Shared data and spec come from the hub, vendored in `shared/` | Accepted |
| [0004](0004-date-representation-and-conversion-algorithm.md) | Date representation, tables and lookups (performance design) | Accepted |
| [0005](0005-no-std-core-with-optional-features.md) | `no_std`, zero-dependency core with opt-in features; the patro in `nepdate-patro` | Accepted |
| [0006](0006-codegen-via-xtask-checked-in.md) | Generate Rust tables with `cargo xtask`, commit the output | Accepted |
| [0007](0007-error-handling-and-api-conventions.md) | Error handling and public API conventions | Accepted |
| [0008](0008-widget-scope.md) | Widget scope (minimal) and use of NepDateWidget | Accepted |
| [0009](0009-csharp-parity-and-deliberate-deviations.md) | Parity with C# NepDate: Rust names for the shared deviations | Accepted |
| [0010](0010-gui-framework-slint.md) | GUI framework for the widget: Slint | Accepted |
| [0011](0011-time-and-time-zones-jiff.md) | Time and time zones: `jiff` in the widget, nothing in the core | Accepted |
| [0012](0012-packaging-signing-and-releases.md) | Packaging, signing and releases | Accepted |
| [0013](0013-licensing.md) | Licensing of this repository (MIT) | Accepted |
| [0014](0014-testing-strategy-and-quality-gates.md) | Testing strategy and quality gates | Accepted |
| [0015](0015-widget-architecture.md) | Widget internal architecture | Accepted |
| [0016](0016-windows-taskbar-text.md) | Windows taskbar text (amends 0008 and 0015) | Accepted |
| [0017](0017-desktop-app-name-mitibar.md) | The desktop app is called MitiBar (amends 0008, 0012 and 0015) | Accepted |
| [0018](0018-software-renderer-and-private-bytes.md) | Software renderer by default, idle memory judged on private bytes (amends 0008 and 0010) | Accepted, conditional on the Devanagari check |
