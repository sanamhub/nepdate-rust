//! Zero-allocation check for the benchmark operations (L2-09 for B1 and B2, L3-06 for B3, B4 and
//! the other phase 2 operations; BENCHMARK §2). Runs without the
//! test harness, on one thread, so nothing else allocates while the counter is read.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// A global allocator can only be written with `unsafe`; the libraries themselves forbid it.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use nepdate::{MonthOverflow, NepaliDate, NepaliDateRange};

/// Wraps the system allocator and adds up every requested size.
struct Counting;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards every call to `System` unchanged; only adds to a counter.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATED.fetch_add(new_size, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// The 64 dates of BENCHMARK §2: serial 36161 (BS 2000-01-01) plus `i * 527`.
fn benchmark_dates() -> [NepaliDate; 64] {
    let mut out = [NepaliDate::MIN; 64];
    let mut date = NepaliDate::MIN;
    let mut serial = 0u32;
    for (i, slot) in out.iter_mut().enumerate() {
        let target = 36_161 + u32::try_from(i).unwrap() * 527;
        while serial < target {
            date = date.succ().unwrap();
            serial += 1;
        }
        *slot = date;
    }
    assert_eq!(out[0].to_string(), "2000/01/01");
    out
}

fn main() {
    let dates = benchmark_dates();
    let parts: [(u16, u8, u8); 64] = dates.map(|d| (d.year(), d.month_number(), d.day()));
    let ad: [(i32, u8, u8); 64] = dates.map(|d| {
        let g = d.to_gregorian();
        (g.year(), g.month(), g.day())
    });

    let before = ALLOCATED.load(Ordering::Relaxed);
    for _ in 0..1_000 {
        for &(y, m, d) in &parts {
            // B1: BS to AD.
            let _ = black_box(NepaliDate::new(black_box(y), m, d).map(NepaliDate::to_gregorian));
        }
        for &(y, m, d) in &ad {
            // B2: AD to BS.
            let _ = black_box(NepaliDate::from_gregorian(black_box(y), m, d));
        }
    }
    let after = ALLOCATED.load(Ordering::Relaxed);
    if after != before {
        eprintln!("no_alloc: B1/B2 allocated {} bytes", after - before);
        std::process::exit(1);
    }
    println!("no_alloc: B1 and B2 allocated 0 bytes over 128,000 calls");

    let before = ALLOCATED.load(Ordering::Relaxed);
    let mut walked = 0usize;
    for _ in 0..1_000 {
        for &date in &dates {
            let date = black_box(date);
            // B3: add 1000 days.
            let _ = black_box(date.checked_add_days(black_box(1_000)));
            // B4: add 13 months, Clamp.
            let _ = black_box(date.checked_add_months(black_box(13), MonthOverflow::Clamp));
            let _ = black_box(date.diff(black_box(dates[63])));
            let _ = black_box(date.fiscal_year().range());
        }
    }
    for _ in 0..10 {
        for day in NepaliDateRange::year(black_box(2081)).unwrap().days() {
            black_box(day);
            walked += 1;
        }
    }
    let after = ALLOCATED.load(Ordering::Relaxed);
    if after != before {
        eprintln!(
            "no_alloc: phase 2 operations allocated {} bytes",
            after - before
        );
        std::process::exit(1);
    }
    assert_eq!(walked, 3_660);
    println!(
        "no_alloc: B3, B4, diff, fiscal_year and a NepaliDateRange::year(2081) walk allocated 0 bytes"
    );
}
