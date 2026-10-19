//! criterion groups `b1_` to `b8_` of `shared/spec/BENCHMARK.md` §3: our operations plus the
//! four competitor crates for B1 and B2. Run with `cargo xtask bench`, which pins the
//! process to logical CPU 2.
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

use bikram::bikram::Bikram;
use nepdate::{Lang, LongDate, MonthOverflow, NepaliDate};
use nepdate_bench::inputs::INPUTS;
use nepdate_bench::{StackBuf, read_day, read_month};

/// `adtobs` covers BS 2000-2090, whose AD span is 1944-01-01..=2033-12-31, so the first AD input
/// (1943-04-14, from BS 2000-01-01) and the last (2034, from BS 2090-12-30) are out of its range.
fn adtobs_ad_inputs(ad: &[(i32, u8, u8); 64]) -> Vec<(i32, u8, u8)> {
    ad.iter()
        .copied()
        .filter(|&(y, _, _)| (1944..=2033).contains(&y))
        .collect()
}

fn bench(c: &mut Criterion) {
    let inputs = &*INPUTS;
    let bs_hyphen = inputs.bs_hyphen();
    let ad_hyphen = inputs.ad_hyphen();
    let adtobs_ad = adtobs_ad_inputs(&inputs.ad);
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
    group.bench_function("npdatetime", |b| {
        b.iter(|| {
            for &(y, m, d) in &inputs.parts {
                black_box(
                    npdatetime::NepaliDate::new(black_box(i32::from(y)), m, d)
                        .unwrap()
                        .to_gregorian()
                        .unwrap(),
                );
            }
        });
    });
    group.bench_function("adtobs", |b| {
        b.iter(|| {
            for &(y, m, d) in &inputs.parts {
                black_box(
                    adtobs::NepaliDate::from_bs_ymd(
                        black_box(i32::from(y)),
                        u32::from(m),
                        u32::from(d),
                    )
                    .unwrap()
                    .to_gregorian(),
                );
            }
        });
    });
    group.bench_function("nepali-date-converter (includes string parsing)", |b| {
        // Warm the once_cell JSON parse before timing; only the conversions are measured.
        black_box(nepali_date_converter::bs_to_ad(&bs_hyphen[0]));
        b.iter(|| {
            for s in &bs_hyphen {
                black_box(nepali_date_converter::bs_to_ad(black_box(s)).unwrap());
            }
        });
    });
    group.bench_function("nepdate-cli", |b| {
        b.iter(|| {
            for &(y, m, d) in &inputs.parts {
                black_box(Bikram::new().to_gregorian(
                    black_box(i32::from(y)),
                    i32::from(m),
                    i32::from(d),
                ));
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
    group.bench_function("npdatetime", |b| {
        b.iter(|| {
            for &(y, m, d) in &inputs.ad {
                black_box(npdatetime::NepaliDate::from_gregorian(black_box(y), m, d).unwrap());
            }
        });
    });
    group.bench_function("adtobs", |b| {
        // Run over the 62 inputs its 1944..=2033 AD range accepts (see `adtobs_ad_inputs`).
        b.iter(|| {
            for &(y, m, d) in &adtobs_ad {
                black_box(
                    adtobs::NepaliDate::from_gregorian_ymd(
                        black_box(y),
                        u32::from(m),
                        u32::from(d),
                    )
                    .unwrap(),
                );
            }
        });
    });
    group.bench_function("nepali-date-converter (includes string parsing)", |b| {
        black_box(nepali_date_converter::ad_to_bs(&ad_hyphen[0]));
        b.iter(|| {
            for s in &ad_hyphen {
                black_box(nepali_date_converter::ad_to_bs(black_box(s)).unwrap());
            }
        });
    });
    group.bench_function("nepdate-cli", |b| {
        b.iter(|| {
            for &(y, m, d) in &inputs.ad {
                let mut b = Bikram::new();
                b.from_gregorian(black_box(y), i32::from(m), i32::from(d));
                black_box((b.get_year(), b.get_month(), b.get_day()));
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
