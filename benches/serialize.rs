//! Serialization benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

#[macro_use]
mod common;

use binostr::{capnp, cbor, dannypack, json, notepack, proto};

fn bench_serialize_single(c: &mut Criterion) {
    let events = common::load_sample(1000);

    if events.is_empty() {
        eprintln!("No events loaded, skipping benchmarks");
        return;
    }

    let mut group = c.benchmark_group("serialize_single");
    group.throughput(Throughput::Elements(1));

    // Sample event for single serialization
    let event = &events[0];

    bench_if_enabled!(group, "json", |b| {
        b.iter(|| json::serialize(black_box(event)))
    });

    bench_if_enabled!(group, "cbor_schemaless", |b| {
        b.iter(|| cbor::schemaless::serialize(black_box(event)))
    });

    bench_if_enabled!(group, "cbor_packed", |b| {
        b.iter(|| cbor::packed::serialize(black_box(event)))
    });

    bench_if_enabled!(group, "cbor_intkey", |b| {
        b.iter(|| cbor::intkey::serialize(black_box(event)))
    });

    bench_if_enabled!(group, "proto_string", |b| {
        b.iter(|| proto::string::serialize(black_box(event)))
    });

    bench_if_enabled!(group, "proto_binary", |b| {
        b.iter(|| proto::binary::serialize(black_box(event)))
    });

    bench_if_enabled!(group, "capnp", |b| {
        b.iter(|| capnp::serialize_event(black_box(event)))
    });

    bench_if_enabled!(group, "capnp_packed", |b| {
        b.iter(|| capnp::serialize_event_packed(black_box(event)))
    });

    bench_if_enabled!(group, "dannypack", |b| {
        b.iter(|| dannypack::serialize_to_vec(black_box(event)))
    });

    bench_if_enabled!(group, "notepack", |b| {
        b.iter(|| notepack::serialize(black_box(event)))
    });

    group.finish();
}

fn bench_serialize_batch(c: &mut Criterion) {
    let events = common::load_sample(1000);

    if events.is_empty() {
        eprintln!("No events loaded, skipping benchmarks");
        return;
    }

    let mut group = c.benchmark_group("serialize_batch");

    for batch_size in [10, 100, 1000] {
        let batch: Vec<_> = events.iter().take(batch_size).cloned().collect();
        if batch.len() < batch_size {
            continue;
        }

        group.throughput(Throughput::Elements(batch_size as u64));

        if common::is_enabled("json") {
            group.bench_with_input(BenchmarkId::new("json", batch_size), &batch, |b, batch| {
                b.iter(|| json::serialize_batch(black_box(batch)))
            });
        }

        if common::is_enabled("cbor_schemaless") {
            group.bench_with_input(
                BenchmarkId::new("cbor_schemaless", batch_size),
                &batch,
                |b, batch| b.iter(|| cbor::schemaless::serialize_batch(black_box(batch))),
            );
        }

        if common::is_enabled("cbor_packed") {
            group.bench_with_input(
                BenchmarkId::new("cbor_packed", batch_size),
                &batch,
                |b, batch| b.iter(|| cbor::packed::serialize_batch(black_box(batch))),
            );
        }

        if common::is_enabled("cbor_intkey") {
            group.bench_with_input(
                BenchmarkId::new("cbor_intkey", batch_size),
                &batch,
                |b, batch| b.iter(|| cbor::intkey::serialize_batch(black_box(batch))),
            );
        }

        if common::is_enabled("proto_string") {
            group.bench_with_input(
                BenchmarkId::new("proto_string", batch_size),
                &batch,
                |b, batch| b.iter(|| proto::string::serialize_batch(black_box(batch))),
            );
        }

        if common::is_enabled("proto_binary") {
            group.bench_with_input(
                BenchmarkId::new("proto_binary", batch_size),
                &batch,
                |b, batch| b.iter(|| proto::binary::serialize_batch(black_box(batch))),
            );
        }

        if common::is_enabled("capnp") {
            group.bench_with_input(BenchmarkId::new("capnp", batch_size), &batch, |b, batch| {
                b.iter(|| capnp::serialize_batch(black_box(batch)))
            });
        }

        if common::is_enabled("capnp_packed") {
            group.bench_with_input(
                BenchmarkId::new("capnp_packed", batch_size),
                &batch,
                |b, batch| b.iter(|| capnp::serialize_batch_packed(black_box(batch))),
            );
        }

        if common::is_enabled("dannypack") {
            group.bench_with_input(
                BenchmarkId::new("dannypack", batch_size),
                &batch,
                |b, batch| b.iter(|| dannypack::serialize_batch(black_box(batch))),
            );
        }

        if common::is_enabled("notepack") {
            group.bench_with_input(
                BenchmarkId::new("notepack", batch_size),
                &batch,
                |b, batch| b.iter(|| notepack::serialize_batch(black_box(batch))),
            );
        }
    }

    group.finish();
}

/// Throughput benchmark measuring events/second (fair comparison across formats)
///
/// Uses Throughput::Elements to measure events processed per second,
/// which provides fair comparison regardless of output size.
fn bench_serialize_throughput(c: &mut Criterion) {
    let events = common::load_sample(1000);

    if events.is_empty() {
        eprintln!("No events loaded, skipping benchmarks");
        return;
    }

    let mut group = c.benchmark_group("serialize_throughput");

    // Use events/sec for fair comparison across formats
    let event_count = events.len() as u64;
    group.throughput(Throughput::Elements(event_count));

    bench_if_enabled!(group, "json", |b| {
        b.iter(|| {
            for event in &events {
                black_box(json::serialize(event));
            }
        })
    });

    bench_if_enabled!(group, "cbor_schemaless", |b| {
        b.iter(|| {
            for event in &events {
                black_box(cbor::schemaless::serialize(event));
            }
        })
    });

    bench_if_enabled!(group, "cbor_packed", |b| {
        b.iter(|| {
            for event in &events {
                black_box(cbor::packed::serialize(event));
            }
        })
    });

    bench_if_enabled!(group, "cbor_intkey", |b| {
        b.iter(|| {
            for event in &events {
                black_box(cbor::intkey::serialize(event));
            }
        })
    });

    bench_if_enabled!(group, "proto_string", |b| {
        b.iter(|| {
            for event in &events {
                black_box(proto::string::serialize(event));
            }
        })
    });

    bench_if_enabled!(group, "proto_binary", |b| {
        b.iter(|| {
            for event in &events {
                black_box(proto::binary::serialize(event));
            }
        })
    });

    bench_if_enabled!(group, "capnp", |b| {
        b.iter(|| {
            for event in &events {
                black_box(capnp::serialize_event(event));
            }
        })
    });

    bench_if_enabled!(group, "capnp_packed", |b| {
        b.iter(|| {
            for event in &events {
                black_box(capnp::serialize_event_packed(event));
            }
        })
    });

    bench_if_enabled!(group, "dannypack", |b| {
        b.iter(|| {
            for event in &events {
                black_box(dannypack::serialize_to_vec(event));
            }
        })
    });

    bench_if_enabled!(group, "notepack", |b| {
        b.iter(|| {
            for event in &events {
                black_box(notepack::serialize(event));
            }
        })
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = common::auto_criterion();
    targets = bench_serialize_single, bench_serialize_batch, bench_serialize_throughput
}
criterion_main!(benches);
