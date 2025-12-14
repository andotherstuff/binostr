//! Zero-copy and selective field access benchmarks
//!
//! Compares the ability of different formats to read specific fields
//! without fully deserializing the entire event. This is Cap'n Proto's
//! main strength and is important for relay filtering use cases.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

mod common;

use binostr::{capnp, cbor, dannypack, json, proto};

/// Simulate JSON field access by deserializing then accessing field
fn json_read_kind(data: &[u8]) -> u16 {
    let event = json::deserialize(data).unwrap();
    event.kind
}

fn json_read_pubkey(data: &[u8]) -> [u8; 32] {
    let event = json::deserialize(data).unwrap();
    event.pubkey
}

fn json_read_kind_and_pubkey(data: &[u8]) -> (u16, [u8; 32]) {
    let event = json::deserialize(data).unwrap();
    (event.kind, event.pubkey)
}

/// Simulate Proto field access by deserializing then accessing field
fn proto_read_kind(data: &[u8]) -> u16 {
    let event = proto::binary::deserialize(data).unwrap();
    event.kind
}

fn proto_read_pubkey(data: &[u8]) -> [u8; 32] {
    let event = proto::binary::deserialize(data).unwrap();
    event.pubkey
}

fn proto_read_kind_and_pubkey(data: &[u8]) -> (u16, [u8; 32]) {
    let event = proto::binary::deserialize(data).unwrap();
    (event.kind, event.pubkey)
}

/// Simulate CBOR field access by deserializing then accessing field
fn cbor_read_kind(data: &[u8]) -> u16 {
    let event = cbor::packed::deserialize(data).unwrap();
    event.kind
}

fn cbor_read_pubkey(data: &[u8]) -> [u8; 32] {
    let event = cbor::packed::deserialize(data).unwrap();
    event.pubkey
}

fn cbor_read_kind_and_pubkey(data: &[u8]) -> (u16, [u8; 32]) {
    let event = cbor::packed::deserialize(data).unwrap();
    (event.kind, event.pubkey)
}

/// Simulate DannyPack field access by deserializing then accessing field
fn dannypack_read_kind(data: &[u8]) -> u16 {
    let event = dannypack::deserialize(data).unwrap();
    event.kind
}

fn dannypack_read_pubkey(data: &[u8]) -> [u8; 32] {
    let event = dannypack::deserialize(data).unwrap();
    event.pubkey
}

fn dannypack_read_kind_and_pubkey(data: &[u8]) -> (u16, [u8; 32]) {
    let event = dannypack::deserialize(data).unwrap();
    (event.kind, event.pubkey)
}

/// Benchmark reading just the `kind` field
fn bench_read_kind(c: &mut Criterion) {
    let events = common::load_sample(1000);
    if events.is_empty() {
        eprintln!("No events loaded, skipping benchmarks");
        return;
    }

    // Pre-serialize
    let json_data: Vec<_> = events.iter().map(json::serialize).collect();
    let proto_data: Vec<_> = events.iter().map(proto::binary::serialize).collect();
    let cbor_data: Vec<_> = events.iter().map(cbor::packed::serialize).collect();
    let capnp_data: Vec<_> = events.iter().map(capnp::serialize_event).collect();
    let dannypack_data: Vec<_> = events.iter().map(dannypack::serialize_to_vec).collect();

    let mut group = c.benchmark_group("read_kind");
    group.throughput(Throughput::Elements(events.len() as u64));

    group.bench_function("json_full_deserialize", |b| {
        b.iter(|| {
            for data in &json_data {
                black_box(json_read_kind(data));
            }
        })
    });

    group.bench_function("proto_full_deserialize", |b| {
        b.iter(|| {
            for data in &proto_data {
                black_box(proto_read_kind(data));
            }
        })
    });

    group.bench_function("cbor_full_deserialize", |b| {
        b.iter(|| {
            for data in &cbor_data {
                black_box(cbor_read_kind(data));
            }
        })
    });

    group.bench_function("capnp_zero_copy", |b| {
        b.iter(|| {
            for data in &capnp_data {
                black_box(capnp::read_kind(data).unwrap());
            }
        })
    });

    group.bench_function("capnp_full_deserialize", |b| {
        b.iter(|| {
            for data in &capnp_data {
                let event = capnp::deserialize_event(data).unwrap();
                black_box(event.kind);
            }
        })
    });

    group.bench_function("dannypack_full_deserialize", |b| {
        b.iter(|| {
            for data in &dannypack_data {
                black_box(dannypack_read_kind(data));
            }
        })
    });

    group.finish();
}

/// Benchmark reading just the `pubkey` field
fn bench_read_pubkey(c: &mut Criterion) {
    let events = common::load_sample(1000);
    if events.is_empty() {
        eprintln!("No events loaded, skipping benchmarks");
        return;
    }

    // Pre-serialize
    let json_data: Vec<_> = events.iter().map(json::serialize).collect();
    let proto_data: Vec<_> = events.iter().map(proto::binary::serialize).collect();
    let cbor_data: Vec<_> = events.iter().map(cbor::packed::serialize).collect();
    let capnp_data: Vec<_> = events.iter().map(capnp::serialize_event).collect();
    let dannypack_data: Vec<_> = events.iter().map(dannypack::serialize_to_vec).collect();

    let mut group = c.benchmark_group("read_pubkey");
    group.throughput(Throughput::Elements(events.len() as u64));

    group.bench_function("json_full_deserialize", |b| {
        b.iter(|| {
            for data in &json_data {
                black_box(json_read_pubkey(data));
            }
        })
    });

    group.bench_function("proto_full_deserialize", |b| {
        b.iter(|| {
            for data in &proto_data {
                black_box(proto_read_pubkey(data));
            }
        })
    });

    group.bench_function("cbor_full_deserialize", |b| {
        b.iter(|| {
            for data in &cbor_data {
                black_box(cbor_read_pubkey(data));
            }
        })
    });

    group.bench_function("capnp_zero_copy", |b| {
        b.iter(|| {
            for data in &capnp_data {
                black_box(capnp::read_pubkey(data).unwrap());
            }
        })
    });

    group.bench_function("capnp_full_deserialize", |b| {
        b.iter(|| {
            for data in &capnp_data {
                let event = capnp::deserialize_event(data).unwrap();
                black_box(event.pubkey);
            }
        })
    });

    group.bench_function("dannypack_full_deserialize", |b| {
        b.iter(|| {
            for data in &dannypack_data {
                black_box(dannypack_read_pubkey(data));
            }
        })
    });

    group.finish();
}

/// Benchmark reading both `kind` and `pubkey` fields (common relay filter scenario)
fn bench_read_kind_and_pubkey(c: &mut Criterion) {
    let events = common::load_sample(1000);
    if events.is_empty() {
        eprintln!("No events loaded, skipping benchmarks");
        return;
    }

    // Pre-serialize
    let json_data: Vec<_> = events.iter().map(json::serialize).collect();
    let proto_data: Vec<_> = events.iter().map(proto::binary::serialize).collect();
    let cbor_data: Vec<_> = events.iter().map(cbor::packed::serialize).collect();
    let capnp_data: Vec<_> = events.iter().map(capnp::serialize_event).collect();
    let dannypack_data: Vec<_> = events.iter().map(dannypack::serialize_to_vec).collect();

    let mut group = c.benchmark_group("read_kind_and_pubkey");
    group.throughput(Throughput::Elements(events.len() as u64));

    group.bench_function("json_full_deserialize", |b| {
        b.iter(|| {
            for data in &json_data {
                black_box(json_read_kind_and_pubkey(data));
            }
        })
    });

    group.bench_function("proto_full_deserialize", |b| {
        b.iter(|| {
            for data in &proto_data {
                black_box(proto_read_kind_and_pubkey(data));
            }
        })
    });

    group.bench_function("cbor_full_deserialize", |b| {
        b.iter(|| {
            for data in &cbor_data {
                black_box(cbor_read_kind_and_pubkey(data));
            }
        })
    });

    group.bench_function("capnp_zero_copy", |b| {
        b.iter(|| {
            for data in &capnp_data {
                black_box(capnp::read_kind_and_pubkey(data).unwrap());
            }
        })
    });

    group.bench_function("capnp_full_deserialize", |b| {
        b.iter(|| {
            for data in &capnp_data {
                let event = capnp::deserialize_event(data).unwrap();
                black_box((event.kind, event.pubkey));
            }
        })
    });

    group.bench_function("dannypack_full_deserialize", |b| {
        b.iter(|| {
            for data in &dannypack_data {
                black_box(dannypack_read_kind_and_pubkey(data));
            }
        })
    });

    group.finish();
}

/// Benchmark full deserialize vs zero-copy for filtering scenario
/// (read kind+pubkey, only deserialize if matches filter)
fn bench_filter_scenario(c: &mut Criterion) {
    let events = common::load_sample(1000);
    if events.is_empty() {
        eprintln!("No events loaded, skipping benchmarks");
        return;
    }

    // Pre-serialize
    let capnp_data: Vec<_> = events.iter().map(capnp::serialize_event).collect();
    let proto_data: Vec<_> = events.iter().map(proto::binary::serialize).collect();

    // Filter: kind=1 and specific pubkey (simulate relay filtering)
    let target_pubkey = events[0].pubkey;
    let target_kind = 1u16;

    let mut group = c.benchmark_group("filter_scenario");
    group.throughput(Throughput::Elements(events.len() as u64));

    // Cap'n Proto: zero-copy check, only deserialize if match
    group.bench_function("capnp_zero_copy_filter", |b| {
        b.iter(|| {
            let mut matched = Vec::new();
            for data in &capnp_data {
                let (kind, pubkey) = capnp::read_kind_and_pubkey(data).unwrap();
                if kind == target_kind && pubkey == target_pubkey {
                    // Only fully deserialize if needed
                    matched.push(capnp::deserialize_event(data).unwrap());
                }
            }
            black_box(matched)
        })
    });

    // Cap'n Proto: always full deserialize
    group.bench_function("capnp_full_deserialize_filter", |b| {
        b.iter(|| {
            let mut matched = Vec::new();
            for data in &capnp_data {
                let event = capnp::deserialize_event(data).unwrap();
                if event.kind == target_kind && event.pubkey == target_pubkey {
                    matched.push(event);
                }
            }
            black_box(matched)
        })
    });

    // Proto: always full deserialize
    group.bench_function("proto_full_deserialize_filter", |b| {
        b.iter(|| {
            let mut matched = Vec::new();
            for data in &proto_data {
                let event = proto::binary::deserialize(data).unwrap();
                if event.kind == target_kind && event.pubkey == target_pubkey {
                    matched.push(event);
                }
            }
            black_box(matched)
        })
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = common::auto_criterion();
    targets = bench_read_kind, bench_read_pubkey, bench_read_kind_and_pubkey, bench_filter_scenario
}
criterion_main!(benches);
