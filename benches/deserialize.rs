//! Deserialization benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

#[macro_use]
mod common;

use binostr::{capnp, cbor, dannypack, json, notepack, proto};

fn bench_deserialize_single(c: &mut Criterion) {
    let events = common::load_sample(1000);

    if events.is_empty() {
        eprintln!("No events loaded, skipping benchmarks");
        return;
    }

    let event = &events[0];

    // Pre-serialize for deserialization benchmarks
    let json_data = json::serialize(event);
    let cbor_schemaless_data = cbor::schemaless::serialize(event);
    let cbor_packed_data = cbor::packed::serialize(event);
    let cbor_intkey_data = cbor::intkey::serialize(event);
    let proto_string_data = proto::string::serialize(event);
    let proto_binary_data = proto::binary::serialize(event);
    let capnp_data = capnp::serialize_event(event);
    let capnp_packed_data = capnp::serialize_event_packed(event);
    let dannypack_data = dannypack::serialize_to_vec(event);
    let notepack_data = notepack::serialize(event);

    let mut group = c.benchmark_group("deserialize_single");
    group.throughput(Throughput::Elements(1));

    bench_if_enabled!(group, "json", |b| {
        b.iter(|| json::deserialize(black_box(&json_data)))
    });

    bench_if_enabled!(group, "cbor_schemaless", |b| {
        b.iter(|| cbor::schemaless::deserialize(black_box(&cbor_schemaless_data)))
    });

    bench_if_enabled!(group, "cbor_packed", |b| {
        b.iter(|| cbor::packed::deserialize(black_box(&cbor_packed_data)))
    });

    bench_if_enabled!(group, "cbor_intkey", |b| {
        b.iter(|| cbor::intkey::deserialize(black_box(&cbor_intkey_data)))
    });

    bench_if_enabled!(group, "proto_string", |b| {
        b.iter(|| proto::string::deserialize(black_box(&proto_string_data)))
    });

    bench_if_enabled!(group, "proto_binary", |b| {
        b.iter(|| proto::binary::deserialize(black_box(&proto_binary_data)))
    });

    bench_if_enabled!(group, "capnp", |b| {
        b.iter(|| capnp::deserialize_event(black_box(&capnp_data)))
    });

    bench_if_enabled!(group, "capnp_packed", |b| {
        b.iter(|| capnp::deserialize_event_packed(black_box(&capnp_packed_data)))
    });

    bench_if_enabled!(group, "dannypack", |b| {
        b.iter(|| dannypack::deserialize(black_box(&dannypack_data)))
    });

    bench_if_enabled!(group, "notepack", |b| {
        b.iter(|| notepack::deserialize(black_box(&notepack_data)))
    });

    group.finish();
}

fn bench_deserialize_batch(c: &mut Criterion) {
    let events = common::load_sample(1000);

    if events.is_empty() {
        eprintln!("No events loaded, skipping benchmarks");
        return;
    }

    let mut group = c.benchmark_group("deserialize_batch");

    for batch_size in [10, 100, 1000] {
        let batch: Vec<_> = events.iter().take(batch_size).cloned().collect();
        if batch.len() < batch_size {
            continue;
        }

        // Pre-serialize batches
        let json_data = json::serialize_batch(&batch);
        let cbor_schemaless_data = cbor::schemaless::serialize_batch(&batch);
        let cbor_packed_data = cbor::packed::serialize_batch(&batch);
        let cbor_intkey_data = cbor::intkey::serialize_batch(&batch);
        let proto_string_data = proto::string::serialize_batch(&batch);
        let proto_binary_data = proto::binary::serialize_batch(&batch);
        let capnp_data = capnp::serialize_batch(&batch);
        let capnp_packed_data = capnp::serialize_batch_packed(&batch);
        let dannypack_data = dannypack::serialize_batch(&batch);
        let notepack_data = notepack::serialize_batch(&batch);

        group.throughput(Throughput::Elements(batch_size as u64));

        if common::is_enabled("json") {
            group.bench_with_input(
                BenchmarkId::new("json", batch_size),
                &json_data,
                |b, data| b.iter(|| json::deserialize_batch(black_box(data))),
            );
        }

        if common::is_enabled("cbor_schemaless") {
            group.bench_with_input(
                BenchmarkId::new("cbor_schemaless", batch_size),
                &cbor_schemaless_data,
                |b, data| b.iter(|| cbor::schemaless::deserialize_batch(black_box(data))),
            );
        }

        if common::is_enabled("cbor_packed") {
            group.bench_with_input(
                BenchmarkId::new("cbor_packed", batch_size),
                &cbor_packed_data,
                |b, data| b.iter(|| cbor::packed::deserialize_batch(black_box(data))),
            );
        }

        if common::is_enabled("cbor_intkey") {
            group.bench_with_input(
                BenchmarkId::new("cbor_intkey", batch_size),
                &cbor_intkey_data,
                |b, data| b.iter(|| cbor::intkey::deserialize_batch(black_box(data))),
            );
        }

        if common::is_enabled("proto_string") {
            group.bench_with_input(
                BenchmarkId::new("proto_string", batch_size),
                &proto_string_data,
                |b, data| b.iter(|| proto::string::deserialize_batch(black_box(data))),
            );
        }

        if common::is_enabled("proto_binary") {
            group.bench_with_input(
                BenchmarkId::new("proto_binary", batch_size),
                &proto_binary_data,
                |b, data| b.iter(|| proto::binary::deserialize_batch(black_box(data))),
            );
        }

        if common::is_enabled("capnp") {
            group.bench_with_input(
                BenchmarkId::new("capnp", batch_size),
                &capnp_data,
                |b, data| b.iter(|| capnp::deserialize_batch(black_box(data))),
            );
        }

        if common::is_enabled("capnp_packed") {
            group.bench_with_input(
                BenchmarkId::new("capnp_packed", batch_size),
                &capnp_packed_data,
                |b, data| b.iter(|| capnp::deserialize_batch_packed(black_box(data))),
            );
        }

        if common::is_enabled("dannypack") {
            group.bench_with_input(
                BenchmarkId::new("dannypack", batch_size),
                &dannypack_data,
                |b, data| b.iter(|| dannypack::deserialize_batch(black_box(data))),
            );
        }

        if common::is_enabled("notepack") {
            group.bench_with_input(
                BenchmarkId::new("notepack", batch_size),
                &notepack_data,
                |b, data| b.iter(|| notepack::deserialize_batch(black_box(data))),
            );
        }
    }

    group.finish();
}

/// Throughput benchmark measuring events/second (fair comparison across formats)
///
/// Uses Throughput::Elements to measure events processed per second,
/// which provides fair comparison regardless of wire format size.
fn bench_deserialize_throughput(c: &mut Criterion) {
    let events = common::load_sample(1000);

    if events.is_empty() {
        eprintln!("No events loaded, skipping benchmarks");
        return;
    }

    // Pre-serialize all events
    let json_data: Vec<_> = events.iter().map(json::serialize).collect();
    let cbor_schemaless_data: Vec<_> = events.iter().map(cbor::schemaless::serialize).collect();
    let cbor_packed_data: Vec<_> = events.iter().map(cbor::packed::serialize).collect();
    let cbor_intkey_data: Vec<_> = events.iter().map(cbor::intkey::serialize).collect();
    let proto_string_data: Vec<_> = events.iter().map(proto::string::serialize).collect();
    let proto_binary_data: Vec<_> = events.iter().map(proto::binary::serialize).collect();
    let capnp_data: Vec<_> = events.iter().map(capnp::serialize_event).collect();
    let capnp_packed_data: Vec<_> = events.iter().map(capnp::serialize_event_packed).collect();
    let dannypack_data: Vec<_> = events.iter().map(dannypack::serialize_to_vec).collect();
    let notepack_data: Vec<_> = events.iter().map(notepack::serialize).collect();

    let event_count = events.len() as u64;

    // Use events/sec for fair comparison across formats with different wire sizes
    let mut group = c.benchmark_group("deserialize_throughput");
    group.throughput(Throughput::Elements(event_count));

    bench_if_enabled!(group, "json", |b| {
        b.iter(|| {
            for data in &json_data {
                black_box(json::deserialize(data).unwrap());
            }
        })
    });

    bench_if_enabled!(group, "cbor_schemaless", |b| {
        b.iter(|| {
            for data in &cbor_schemaless_data {
                black_box(cbor::schemaless::deserialize(data).unwrap());
            }
        })
    });

    bench_if_enabled!(group, "cbor_packed", |b| {
        b.iter(|| {
            for data in &cbor_packed_data {
                black_box(cbor::packed::deserialize(data).unwrap());
            }
        })
    });

    bench_if_enabled!(group, "cbor_intkey", |b| {
        b.iter(|| {
            for data in &cbor_intkey_data {
                black_box(cbor::intkey::deserialize(data).unwrap());
            }
        })
    });

    bench_if_enabled!(group, "proto_string", |b| {
        b.iter(|| {
            for data in &proto_string_data {
                black_box(proto::string::deserialize(data).unwrap());
            }
        })
    });

    bench_if_enabled!(group, "proto_binary", |b| {
        b.iter(|| {
            for data in &proto_binary_data {
                black_box(proto::binary::deserialize(data).unwrap());
            }
        })
    });

    bench_if_enabled!(group, "capnp", |b| {
        b.iter(|| {
            for data in &capnp_data {
                black_box(capnp::deserialize_event(data).unwrap());
            }
        })
    });

    bench_if_enabled!(group, "capnp_packed", |b| {
        b.iter(|| {
            for data in &capnp_packed_data {
                black_box(capnp::deserialize_event_packed(data).unwrap());
            }
        })
    });

    bench_if_enabled!(group, "dannypack", |b| {
        b.iter(|| {
            for data in &dannypack_data {
                black_box(dannypack::deserialize(data).unwrap());
            }
        })
    });

    bench_if_enabled!(group, "notepack", |b| {
        b.iter(|| {
            for data in &notepack_data {
                black_box(notepack::deserialize(data).unwrap());
            }
        })
    });

    group.finish();
}

/// Throughput benchmark measuring bytes/second for each format
///
/// This measures actual wire bytes processed per second for each format,
/// useful for understanding how formats perform relative to their own size.
fn bench_deserialize_bytes_throughput(c: &mut Criterion) {
    let events = common::load_sample(1000);

    if events.is_empty() {
        eprintln!("No events loaded, skipping benchmarks");
        return;
    }

    // Pre-serialize all events and compute format-specific sizes
    let json_data: Vec<_> = events.iter().map(json::serialize).collect();
    let cbor_schemaless_data: Vec<_> = events.iter().map(cbor::schemaless::serialize).collect();
    let cbor_packed_data: Vec<_> = events.iter().map(cbor::packed::serialize).collect();
    let cbor_intkey_data: Vec<_> = events.iter().map(cbor::intkey::serialize).collect();
    let proto_string_data: Vec<_> = events.iter().map(proto::string::serialize).collect();
    let proto_binary_data: Vec<_> = events.iter().map(proto::binary::serialize).collect();
    let capnp_data: Vec<_> = events.iter().map(capnp::serialize_event).collect();
    let capnp_packed_data: Vec<_> = events.iter().map(capnp::serialize_event_packed).collect();
    let dannypack_data: Vec<_> = events.iter().map(dannypack::serialize_to_vec).collect();
    let notepack_data: Vec<_> = events.iter().map(notepack::serialize).collect();

    // Helper to run benchmark with format-specific byte throughput (if enabled)
    macro_rules! bench_with_bytes {
        ($group:expr, $name:expr, $data:expr, $deserialize:expr) => {{
            if common::is_enabled($name) {
                let total_bytes: usize = $data.iter().map(|d| d.len()).sum();
                let mut group = c.benchmark_group(format!("deserialize_bytes/{}", $name));
                group.throughput(Throughput::Bytes(total_bytes as u64));
                group.bench_function($name, |b| {
                    b.iter(|| {
                        for data in &$data {
                            black_box($deserialize(data).unwrap());
                        }
                    })
                });
                group.finish();
            }
        }};
    }

    bench_with_bytes!(c, "json", json_data, json::deserialize);
    bench_with_bytes!(c, "cbor_schemaless", cbor_schemaless_data, cbor::schemaless::deserialize);
    bench_with_bytes!(c, "cbor_packed", cbor_packed_data, cbor::packed::deserialize);
    bench_with_bytes!(c, "cbor_intkey", cbor_intkey_data, cbor::intkey::deserialize);
    bench_with_bytes!(c, "proto_string", proto_string_data, proto::string::deserialize);
    bench_with_bytes!(c, "proto_binary", proto_binary_data, proto::binary::deserialize);
    bench_with_bytes!(c, "capnp", capnp_data, capnp::deserialize_event);
    bench_with_bytes!(c, "capnp_packed", capnp_packed_data, capnp::deserialize_event_packed);
    bench_with_bytes!(c, "dannypack", dannypack_data, dannypack::deserialize);
    bench_with_bytes!(c, "notepack", notepack_data, notepack::deserialize);
}

criterion_group! {
    name = benches;
    config = common::auto_criterion();
    targets = bench_deserialize_single, bench_deserialize_batch, bench_deserialize_throughput, bench_deserialize_bytes_throughput
}
criterion_main!(benches);
