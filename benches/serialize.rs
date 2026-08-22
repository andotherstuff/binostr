//! Serialization benchmarks driven by the shared format registry.

use binostr::stats::{self, Format};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

mod common;

fn bench_serialize_single(c: &mut Criterion) {
    let events = common::load_sample(1000);
    let Some(event) = events.first() else { return };
    let mut group = c.benchmark_group("serialize_single");
    group.throughput(Throughput::Elements(1));
    for format in Format::enabled() {
        group.bench_function(format.short_name(), |b| {
            b.iter(|| stats::serialize(black_box(event), format))
        });
    }
    group.finish();
}

fn bench_serialize_batch(c: &mut Criterion) {
    let events = common::load_sample(1000);
    let mut group = c.benchmark_group("serialize_batch");
    for batch_size in [10, 100, 1000] {
        let batch = &events[..events.len().min(batch_size)];
        if batch.len() != batch_size {
            continue;
        }
        group.throughput(Throughput::Elements(batch_size as u64));
        for format in Format::enabled() {
            group.bench_with_input(
                BenchmarkId::new(format.short_name(), batch_size),
                &batch,
                |b, batch| b.iter(|| stats::serialize_batch(black_box(batch), format)),
            );
        }
    }
    group.finish();
}

fn bench_serialize_throughput(c: &mut Criterion) {
    let events = common::load_sample(1000);
    if events.is_empty() {
        return;
    }
    let mut group = c.benchmark_group("serialize_throughput");
    group.throughput(Throughput::Elements(events.len() as u64));
    for format in Format::enabled() {
        group.bench_function(format.short_name(), |b| {
            b.iter(|| {
                for event in &events {
                    black_box(stats::serialize(event, format));
                }
            })
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = common::auto_criterion();
    targets = bench_serialize_single, bench_serialize_batch, bench_serialize_throughput
}
criterion_main!(benches);
