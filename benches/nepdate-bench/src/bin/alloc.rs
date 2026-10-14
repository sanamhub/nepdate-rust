//! Counts every allocation made by each B-operation of `shared/spec/BENCHMARK.md`, 10,000 rounds
//! over the 64 inputs per operation (L6-02 AC2). The gate is the `no_alloc` tests; this binary
//! only fills the `docs/benchmarks.md` table.
//!
//! A global allocator can only be written with `unsafe`; the libraries themselves forbid it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use nepdate::{Lang, LongDate, MonthOverflow, NepaliDate};
use nepdate_bench::inputs::INPUTS;
use nepdate_bench::{StackBuf, read_day, read_month};

/// Wraps the system allocator and counts every call and byte.
struct Counting;

static CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards every call to `System` unchanged; only adds to two counters.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        CALLS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        CALLS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(new_size, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn snapshot() -> (usize, usize) {
    (CALLS.load(Ordering::Relaxed), BYTES.load(Ordering::Relaxed))
}

/// Runs `f`, then prints `name: calls, bytes` as the delta since the previous snapshot.
fn measure(name: &str, f: impl FnOnce()) {
    let (c0, b0) = snapshot();
    f();
    let (c1, b1) = snapshot();
    println!("{name}: {} calls, {} bytes", c1 - c0, b1 - b0);
}

fn main() {
    let inputs = &*INPUTS;
    let weekday = LongDate {
        weekday: true,
        ..LongDate::default()
    };
    let rounds = 10_000;

    measure("B1 bs->ad", || {
        for _ in 0..rounds {
            for &(y, m, d) in &inputs.parts {
                black_box(NepaliDate::new(black_box(y), m, d).unwrap().to_gregorian());
            }
        }
    });

    measure("B2 ad->bs", || {
        for _ in 0..rounds {
            for &(y, m, d) in &inputs.ad {
                black_box(NepaliDate::from_gregorian(black_box(y), m, d).unwrap());
            }
        }
    });

    measure("B3 add 1000 days", || {
        for _ in 0..rounds {
            for &date in &inputs.bs {
                black_box(date.checked_add_days(black_box(1_000)).unwrap());
            }
        }
    });

    measure("B4 add 13 months Clamp", || {
        for _ in 0..rounds {
            for &date in &inputs.bs {
                black_box(
                    date.checked_add_months(black_box(13), MonthOverflow::Clamp)
                        .unwrap(),
                );
            }
        }
    });

    let mut buf = StackBuf::new();
    measure("B5 long write_to", || {
        for _ in 0..rounds {
            for &date in &inputs.bs {
                buf.clear();
                date.long(weekday, Lang::Ne).write_to(&mut buf).unwrap();
                black_box(buf.as_bytes());
            }
        }
    });

    measure("B5 long to_string (allocates, for comparison)", || {
        for _ in 0..rounds {
            for &date in &inputs.bs {
                black_box(date.long(weekday, Lang::Ne).to_string());
            }
        }
    });

    measure("B6 strict parse", || {
        for _ in 0..rounds {
            for s in &inputs.text {
                black_box(black_box(s.as_str()).parse::<NepaliDate>().unwrap());
            }
        }
    });

    measure("B7 patro info", || {
        for _ in 0..rounds {
            let mut total = 0;
            for &date in &inputs.patro {
                total += read_day(black_box(date));
            }
            black_box(total);
        }
    });

    measure("B8 month grid", || {
        for _ in 0..rounds {
            let mut total = 0;
            for &date in &inputs.patro {
                total += read_month(black_box(date));
            }
            black_box(total);
        }
    });
}
