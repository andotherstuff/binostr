//! Benchmarks by event size and tag count categories
//!
//! Tests serialization performance across different event characteristics
//! to understand how formats scale with event complexity.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

mod common;

use binostr::event::{SizeCategory, TagCategory};
use binostr::{capnp, cbor, dannypack, json, notepack, proto, EventSampler, NostrEvent};

const DATA_DIR: &str = "data";
const SAMPLE_SIZE: usize = 100;

/// Load events by size category
fn load_by_size(category: SizeCategory, count: usize) -> Vec<NostrEvent> {
    let mut sampler = match EventSampler::from_directory(DATA_DIR, count * 20) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: Could not load events: {}", e);
            return common::generate_synthetic_events(count);
        }
    };

    let events = sampler.sample_size(category, count);
    if events.is_empty() {
        eprintln!("Warning: No events found for category {:?}", category);
        return Vec::new();
    }

    events.into_iter().cloned().collect()
}

/// Load events by tag category
fn load_by_tags(category: TagCategory, count: usize) -> Vec<NostrEvent> {
    let mut sampler = match EventSampler::from_directory(DATA_DIR, count * 20) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: Could not load events: {}", e);
            return common::generate_synthetic_events(count);
        }
    };

    let events = sampler.sample_tags(category, count);
    if events.is_empty() {
        eprintln!("Warning: No events found for category {:?}", category);
        return Vec::new();
    }

    events.into_iter().cloned().collect()
}

/// Benchmark a specific size category
fn bench_size_category(c: &mut Criterion, category: SizeCategory, name: &str) {
    let events = load_by_size(category, SAMPLE_SIZE);

    if events.is_empty() {
        eprintln!("No events for category {}, skipping", name);
        return;
    }

    let group_name = format!("size_{}", name);
    let mut group = c.benchmark_group(&group_name);
    group.throughput(Throughput::Elements(events.len() as u64));

    // Serialize benchmarks
    group.bench_function("serialize/json", |b| {
        b.iter(|| {
            for event in &events {
                black_box(json::serialize(event));
            }
        })
    });

    group.bench_function("serialize/cbor_packed", |b| {
        b.iter(|| {
            for event in &events {
                black_box(cbor::packed::serialize(event));
            }
        })
    });

    group.bench_function("serialize/proto_binary", |b| {
        b.iter(|| {
            for event in &events {
                black_box(proto::binary::serialize(event));
            }
        })
    });

    group.bench_function("serialize/capnp", |b| {
        b.iter(|| {
            for event in &events {
                black_box(capnp::serialize_event(event));
            }
        })
    });

    group.bench_function("serialize/dannypack", |b| {
        b.iter(|| {
            for event in &events {
                black_box(dannypack::serialize_to_vec(event));
            }
        })
    });

    group.bench_function("serialize/notepack", |b| {
        b.iter(|| {
            for event in &events {
                black_box(notepack::serialize(event));
            }
        })
    });

    // Pre-serialize for deserialization benchmarks
    let json_data: Vec<_> = events.iter().map(json::serialize).collect();
    let cbor_data: Vec<_> = events.iter().map(cbor::packed::serialize).collect();
    let proto_data: Vec<_> = events.iter().map(proto::binary::serialize).collect();
    let capnp_data: Vec<_> = events.iter().map(capnp::serialize_event).collect();
    let dannypack_data: Vec<_> = events.iter().map(dannypack::serialize_to_vec).collect();
    let notepack_data: Vec<_> = events.iter().map(notepack::serialize).collect();

    // Deserialize benchmarks
    group.bench_function("deserialize/json", |b| {
        b.iter(|| {
            for data in &json_data {
                black_box(json::deserialize(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/cbor_packed", |b| {
        b.iter(|| {
            for data in &cbor_data {
                black_box(cbor::packed::deserialize(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/proto_binary", |b| {
        b.iter(|| {
            for data in &proto_data {
                black_box(proto::binary::deserialize(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/capnp", |b| {
        b.iter(|| {
            for data in &capnp_data {
                black_box(capnp::deserialize_event(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/dannypack", |b| {
        b.iter(|| {
            for data in &dannypack_data {
                black_box(dannypack::deserialize(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/notepack", |b| {
        b.iter(|| {
            for data in &notepack_data {
                black_box(notepack::deserialize(data).unwrap());
            }
        })
    });

    group.finish();

    // Print size statistics
    print_size_stats(&events, &group_name);
}

/// Benchmark a specific tag category
fn bench_tag_category(c: &mut Criterion, category: TagCategory, name: &str) {
    let events = load_by_tags(category, SAMPLE_SIZE);

    if events.is_empty() {
        eprintln!("No events for category {}, skipping", name);
        return;
    }

    let group_name = format!("tags_{}", name);
    let mut group = c.benchmark_group(&group_name);
    group.throughput(Throughput::Elements(events.len() as u64));

    // Serialize benchmarks
    group.bench_function("serialize/json", |b| {
        b.iter(|| {
            for event in &events {
                black_box(json::serialize(event));
            }
        })
    });

    group.bench_function("serialize/cbor_packed", |b| {
        b.iter(|| {
            for event in &events {
                black_box(cbor::packed::serialize(event));
            }
        })
    });

    group.bench_function("serialize/proto_binary", |b| {
        b.iter(|| {
            for event in &events {
                black_box(proto::binary::serialize(event));
            }
        })
    });

    group.bench_function("serialize/capnp", |b| {
        b.iter(|| {
            for event in &events {
                black_box(capnp::serialize_event(event));
            }
        })
    });

    group.bench_function("serialize/dannypack", |b| {
        b.iter(|| {
            for event in &events {
                black_box(dannypack::serialize_to_vec(event));
            }
        })
    });

    group.bench_function("serialize/notepack", |b| {
        b.iter(|| {
            for event in &events {
                black_box(notepack::serialize(event));
            }
        })
    });

    // Pre-serialize for deserialization benchmarks
    let json_data: Vec<_> = events.iter().map(json::serialize).collect();
    let cbor_data: Vec<_> = events.iter().map(cbor::packed::serialize).collect();
    let proto_data: Vec<_> = events.iter().map(proto::binary::serialize).collect();
    let capnp_data: Vec<_> = events.iter().map(capnp::serialize_event).collect();
    let dannypack_data: Vec<_> = events.iter().map(dannypack::serialize_to_vec).collect();
    let notepack_data: Vec<_> = events.iter().map(notepack::serialize).collect();

    // Deserialize benchmarks
    group.bench_function("deserialize/json", |b| {
        b.iter(|| {
            for data in &json_data {
                black_box(json::deserialize(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/cbor_packed", |b| {
        b.iter(|| {
            for data in &cbor_data {
                black_box(cbor::packed::deserialize(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/proto_binary", |b| {
        b.iter(|| {
            for data in &proto_data {
                black_box(proto::binary::deserialize(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/capnp", |b| {
        b.iter(|| {
            for data in &capnp_data {
                black_box(capnp::deserialize_event(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/dannypack", |b| {
        b.iter(|| {
            for data in &dannypack_data {
                black_box(dannypack::deserialize(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/notepack", |b| {
        b.iter(|| {
            for data in &notepack_data {
                black_box(notepack::deserialize(data).unwrap());
            }
        })
    });

    group.finish();

    // Print size statistics
    print_size_stats(&events, &group_name);
}

fn print_size_stats(events: &[NostrEvent], category: &str) {
    if events.is_empty() {
        return;
    }

    let n = events.len();

    let json_total: usize = events.iter().map(|e| json::serialize(e).len()).sum();
    let cbor_total: usize = events.iter().map(|e| cbor::packed::serialize(e).len()).sum();
    let proto_total: usize = events.iter().map(|e| proto::binary::serialize(e).len()).sum();
    let capnp_total: usize = events.iter().map(|e| capnp::serialize_event(e).len()).sum();
    let dannypack_total: usize = events.iter().map(|e| dannypack::serialize_to_vec(e).len()).sum();
    let notepack_total: usize = events.iter().map(|e| notepack::serialize(e).len()).sum();

    println!("\n=== {} - {} events ===", category, n);
    println!("Average sizes:");
    println!("  JSON:         {:>6} bytes (100.0%)", json_total / n);
    println!(
        "  CBOR Packed:  {:>6} bytes ({:>5.1}%)",
        cbor_total / n,
        100.0 * cbor_total as f64 / json_total as f64
    );
    println!(
        "  Proto Binary: {:>6} bytes ({:>5.1}%)",
        proto_total / n,
        100.0 * proto_total as f64 / json_total as f64
    );
    println!(
        "  Cap'n Proto:  {:>6} bytes ({:>5.1}%)",
        capnp_total / n,
        100.0 * capnp_total as f64 / json_total as f64
    );
    println!(
        "  DannyPack:    {:>6} bytes ({:>5.1}%)",
        dannypack_total / n,
        100.0 * dannypack_total as f64 / json_total as f64
    );
    println!(
        "  Notepack:     {:>6} bytes ({:>5.1}%)",
        notepack_total / n,
        100.0 * notepack_total as f64 / json_total as f64
    );
}

// Size category benchmarks
fn bench_size_tiny(c: &mut Criterion) {
    bench_size_category(c, SizeCategory::Tiny, "tiny");
}

fn bench_size_small(c: &mut Criterion) {
    bench_size_category(c, SizeCategory::Small, "small");
}

fn bench_size_medium(c: &mut Criterion) {
    bench_size_category(c, SizeCategory::Medium, "medium");
}

fn bench_size_large(c: &mut Criterion) {
    bench_size_category(c, SizeCategory::Large, "large");
}

fn bench_size_huge(c: &mut Criterion) {
    bench_size_category(c, SizeCategory::Huge, "huge");
}

// Tag category benchmarks
fn bench_tags_none(c: &mut Criterion) {
    bench_tag_category(c, TagCategory::None, "none");
}

fn bench_tags_few(c: &mut Criterion) {
    bench_tag_category(c, TagCategory::Few, "few");
}

fn bench_tags_moderate(c: &mut Criterion) {
    bench_tag_category(c, TagCategory::Moderate, "moderate");
}

fn bench_tags_many(c: &mut Criterion) {
    bench_tag_category(c, TagCategory::Many, "many");
}

fn bench_tags_massive(c: &mut Criterion) {
    bench_tag_category(c, TagCategory::Massive, "massive");
}

criterion_group! {
    name = size_benches;
    config = common::auto_criterion();
    targets = bench_size_tiny, bench_size_small, bench_size_medium, bench_size_large, bench_size_huge
}

criterion_group! {
    name = tag_benches;
    config = common::auto_criterion();
    targets = bench_tags_none, bench_tags_few, bench_tags_moderate, bench_tags_many, bench_tags_massive
}

criterion_main!(size_benches, tag_benches);
