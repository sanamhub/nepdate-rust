# Learn 10: Models, structs and repeaters in Slint (phase W2)

## 1. Structs shared between Slint and Rust

```slint
export struct CellData { bs-day: string, ad-day: string, is-today: bool, tint: bool }
export component MonthGrid {
    in property <[CellData]> cells;           // [T] = a model (list)
    GridLayout { for cell[i] in root.cells: Cell { data: cell; row: i / 7; col: mod(i, 7); } }
}
```

Slint generates a Rust `struct CellData { bs_day: SharedString, ... }`.

## 2. Models

```rust
let cells: Vec<CellData> = month_view.cells.iter().map(to_cell_data).collect();
ui.set_cells(slint::ModelRc::new(slint::VecModel::from(cells)));
```

`VecModel` is like `ObservableCollection`: if you keep it and call `set_row_data`,
only that cell re-renders. For 42 cells, replacing the whole model per month is fine and simpler.

## 3. Why the conversion lives in `ui_bridge.rs` only

`app::MonthView` (pure Rust, tested) knows nothing about Slint; `CellData` (generated)
knows nothing about `nepdate`. One small mapping function connects them. Tests cover the
logic without a display, and swapping the UI toolkit touches one file (ADR-0015).

## 4. Callbacks with arguments

```slint
callback day-clicked(int);                    // index of the cell
```
```rust
ui.on_day_clicked(move |index| { /* index: i32 */ });
```
