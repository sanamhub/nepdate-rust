//! Compiles the `.slint` files into Rust before the crate compiles (learn/08 §2).
//! A typo in the markup is a compile error here, with the file and line.

// Build scripts may panic (W0-01); a bad `.slint` file should fail the build loudly.
#![allow(clippy::expect_used, clippy::unwrap_used)]

fn main() {
    slint_build::compile("ui/app.slint").expect("slint compile");
}
