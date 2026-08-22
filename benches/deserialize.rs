//! Deserialization benchmarks driven by the shared format registry.

use binostr::stats::{self, Format};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

mod common;

fn bench_deserialize_single(c: &mut Criterion) {
    let events = common::load_sample(1000);
    let Some(event) = events.first() else { return };
    let mut group = c.benchmark_group("deserialize_single");
    group.throughput(Throughput::Elements(1));
    for format in Format::enabled() {
        let encoded = stats::serialize(event, format);
        group.bench_function(format.short_name(), |b| {
            b.iter(|| stats::deserialize(black_box(&encoded), format).unwrap())
        });
    }
    group.finish();
}

fn bench_deserialize_batch(c: &mut Criterion) {
    let events = common::load_sample(1000);
    let mut group = c.benchmark_group("deserialize_batch");
    for batch_size in [10, 100, 1000] {
        let batch = &events[..events.len().min(batch_size)];
        if batch.len() != batch_size {
            continue;
        }
        group.throughput(Throughput::Elements(batch_size as u64));
        for format in Format::enabled() {
            let encoded = stats::serialize_batch(batch, format);
            group.bench_with_input(
                BenchmarkId::new(format.short_name(), batch_size),
                &encoded,
                |b, data| b.iter(|| stats::deserialize_batch(black_box(data), format).unwrap()),
            );
        }
    }
    group.finish();
}

fn bench_deserialize_throughput(c: &mut Criterion) {
    let events = common::load_sample(1000);
    if events.is_empty() {
        return;
    }
    let mut group = c.benchmark_group("deserialize_throughput");
    group.throughput(Throughput::Elements(events.len() as u64));
    for format in Format::enabled() {
        let encoded: Vec<_> = events
            .iter()
            .map(|event| stats::serialize(event, format))
            .collect();
        group.bench_function(format.short_name(), |b| {
            b.iter(|| {
                for data in &encoded {
                    black_box(stats::deserialize(data, format).unwrap());
                }
            })
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = common::auto_criterion();
    targets = bench_deserialize_single, bench_deserialize_batch, bench_deserialize_throughput
}
criterion_main!(benches);
