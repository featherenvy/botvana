use std::convert::TryFrom;

use criterion::*;
use rust_decimal::prelude::*;

use botvana::market::orderbook::*;

pub fn bench_price_levels_vec(c: &mut Criterion) {
    // let mut group = c.benchmark_group("PriceLevelsVec::update");

    // group.bench_function("owned", |b| {
    //     b.iter_batched(|| update_levels.clone(), |data| levels.update(data), BatchSize::SmallInput)
    // });

    let levels_f64 = (1..100)
        .map(|n| (n as f64, 1.0 as f64))
        .collect::<Vec<(f64, f64)>>();
    let mut levels_f64 = PriceLevelsVec::from_tuples_vec(levels_f64);

    let update_levels = (-25..25)
        .map(|n| (n as f64, 0.0 as f64))
        .collect::<Vec<(f64, f64)>>();
    let update_levels = PriceLevelsVec::from_tuples_vec(update_levels);

    c.bench_with_input(
        BenchmarkId::new("update(f64)", "50 levels"),
        &update_levels,
        |b, l| b.iter(|| levels_f64.update(&l)),
    );

    let levels_dec = (1..100)
        .map(|n| (Decimal::try_from(n).unwrap(), Decimal::ONE))
        .collect::<Vec<(Decimal, Decimal)>>();
    let mut levels_dec = PriceLevelsVec::from_tuples_vec(levels_dec);

    let update_levels = (-50..50)
        .map(|n| (Decimal::try_from(n).unwrap(), Decimal::ZERO))
        .collect::<Vec<(Decimal, Decimal)>>();
    let update_levels = PriceLevelsVec::from_tuples_vec(update_levels);

    c.bench_with_input(
        BenchmarkId::new("update(dec)", "50 levels"),
        &update_levels,
        |b, l| b.iter(|| levels_dec.update(l)),
    );
}

criterion_group!(benches, bench_price_levels_vec);
criterion_main!(benches);
