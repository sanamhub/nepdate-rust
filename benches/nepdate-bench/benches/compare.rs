//! criterion groups `b1_` to `b8_` of `shared/spec/BENCHMARK.md` §3 for this port. Run with
//! `cargo xtask bench`, which pins the process to logical CPU 2.
//!
//! Every iteration runs the operation over all 64 inputs; `Throughput::Elements(64)` makes the
//! report show time per operation. Results are consumed with `std::hint::black_box`.

#![allow(clippy::unwrap_used, clippy::expect_used)]
// One `bench` function registers all eight groups in benchmark order; splitting it would add
// boilerplate without making the per-operation closures shorter.
#![allow(clippy::too_many_lines)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};

use nepdate::{Lang, LongDate, MonthOverflow, NepaliDate};
use nepdate_bench::inputs::INPUTS;
use nepdate_bench::{StackBuf, read_day, read_month};

fn bench(c: &mut Criterion) {
    let inputs = &*INPUTS;
    let weekday = LongDate {
        weekday: true,
        ..LongDate::default()
    };

    let mut group = c.benchmark_group("b1_bs_to_ad");
    group.throughput(Throughput::Elements(64));
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(100);
    group.bench_function("nepdate", |b| {
        b.iter(|| {
            for &(y, m, d) in &inputs.parts {
                black_box(NepaliDate::new(black_box(y), m, d).unwrap().to_gregorian());
            }
        });
    });
    group.finish();

    let mut group = c.benchmark_group("b2_ad_to_bs");
    group.throughput(Throughput::Elements(64));
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(100);
    group.bench_function("nepdate", |b| {
        b.iter(|| {
            for &(y, m, d) in &inputs.ad {
                black_box(NepaliDate::from_gregorian(black_box(y), m, d).unwrap());
            }
        });
    });
    group.finish();

    let mut group = c.benchmark_group("b3_add_1000_days");
    group.throughput(Throughput::Elements(64));
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(100);
    group.bench_function("nepdate", |b| {
        b.iter(|| {
            for &date in &inputs.bs {
                black_box(date.checked_add_days(black_box(1_000)).unwrap());
            }
        });
    });
    group.finish();

    let mut group = c.benchmark_group("b4_add_13_months_clamp");
    group.throughput(Throughput::Elements(64));
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(100);
    group.bench_function("nepdate", |b| {
        b.iter(|| {
            for &date in &inputs.bs {
                black_box(
                    date.checked_add_months(black_box(13), MonthOverflow::Clamp)
                        .unwrap(),
                );
            }
        });
    });
    group.finish();

    let mut group = c.benchmark_group("b5_long_nepali_with_weekday");
    group.throughput(Throughput::Elements(64));
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(100);
    group.bench_function("nepdate write_to", |b| {
        let mut buf = StackBuf::new();
        b.iter(|| {
            for &date in &inputs.bs {
                buf.clear();
                date.long(weekday, Lang::Ne).write_to(&mut buf).unwrap();
                black_box(buf.as_bytes());
            }
        });
    });
    group.bench_function("nepdate to_string (allocates, for comparison)", |b| {
        b.iter(|| {
            for &date in &inputs.bs {
                black_box(date.long(weekday, Lang::Ne).to_string());
            }
        });
    });
    group.finish();

    let mut group = c.benchmark_group("b6_strict_parse");
    group.throughput(Throughput::Elements(64));
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(100);
    group.bench_function("nepdate", |b| {
        b.iter(|| {
            for s in &inputs.text {
                black_box(black_box(s.as_str()).parse::<NepaliDate>().unwrap());
            }
        });
    });
    group.finish();

    let mut group = c.benchmark_group("b7_patro_info");
    group.throughput(Throughput::Elements(64));
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(100);
    group.bench_function("nepdate-patro", |b| {
        b.iter(|| {
            let mut total = 0;
            for &date in &inputs.patro {
                total += read_day(black_box(date));
            }
            black_box(total);
        });
    });
    group.finish();

    let mut group = c.benchmark_group("b8_month_grid");
    group.throughput(Throughput::Elements(64));
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(100);
    group.bench_function("nepdate-patro", |b| {
        b.iter(|| {
            let mut total = 0;
            for &date in &inputs.patro {
                total += read_month(black_box(date));
            }
            black_box(total);
        });
    });
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
