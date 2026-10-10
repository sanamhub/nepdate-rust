//! Zero-allocation check for B7 and B8 (BENCHMARK §2) and a full `public_holidays()` walk
//! (L5-07). Same counter as `crates/nepdate/tests/no_alloc.rs`. Runs without the test harness,
//! on one thread, so nothing else allocates while the counter is read.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// A global allocator can only be written with `unsafe`; the libraries themselves forbid it.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use nepdate::{Lang, NepaliDate, NepaliDateRange};

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

/// The 64 B7/B8 dates of BENCHMARK §2: `c0 + i * ((c1 - c0) / 63)` over the patro coverage.
fn benchmark_dates() -> [NepaliDate; 64] {
    let coverage = nepdate_patro::coverage();
    let step = coverage.start().days_until(coverage.end()) / 63;
    let mut out = [NepaliDate::MIN; 64];
    for (i, slot) in (0i32..).zip(out.iter_mut()) {
        *slot = coverage.start().checked_add_days(i * step).unwrap();
    }
    assert_eq!(out[0], coverage.start());
    out
}

/// B7: everything `info` says about one day, every name read in both languages.
fn read_day(date: NepaliDate) -> usize {
    let info = nepdate_patro::info(black_box(date));
    let mut bytes = 0;
    if let Some(tithi) = info.tithi() {
        bytes += tithi.name(Lang::En).len() + tithi.name(Lang::Ne).len();
    }
    bytes += usize::from(info.is_public_holiday());
    for event in info.events() {
        bytes += event.name(Lang::En).len() + event.name(Lang::Ne).len();
    }
    black_box(bytes)
}

fn main() {
    let dates = benchmark_dates();
    let months: Vec<(u16, u8)> = dates.iter().map(|d| (d.year(), d.month_number())).collect();

    let before = ALLOCATED.load(Ordering::Relaxed);
    let mut read = 0usize;
    for _ in 0..1_000 {
        for &date in &dates {
            read += read_day(date);
        }
    }
    let after = ALLOCATED.load(Ordering::Relaxed);
    if after != before {
        eprintln!("no_alloc: B7 allocated {} bytes", after - before);
        std::process::exit(1);
    }
    assert!(read > 0);
    println!("no_alloc: B7 allocated 0 bytes over 64,000 calls");

    let before = ALLOCATED.load(Ordering::Relaxed);
    let mut days = 0usize;
    for _ in 0..100 {
        for &(year, month) in &months {
            // B8: a month grid.
            for day in NepaliDateRange::month(black_box(year), month)
                .unwrap()
                .days()
            {
                read_day(day);
                days += 1;
            }
        }
    }
    let mut holidays = 0usize;
    for _ in 0..100 {
        for date in nepdate_patro::public_holidays() {
            black_box(date);
            holidays += 1;
        }
        black_box(nepdate_patro::public_holidays().next_back());
    }
    black_box(nepdate_patro::coverage());
    let after = ALLOCATED.load(Ordering::Relaxed);
    if after != before {
        eprintln!(
            "no_alloc: B8 and public_holidays allocated {} bytes",
            after - before
        );
        std::process::exit(1);
    }
    assert_eq!(holidays, 29_400);
    println!("no_alloc: B8 ({days} days) and public_holidays() walks allocated 0 bytes");
}
