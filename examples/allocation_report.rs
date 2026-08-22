//! Measure allocations in a separate process so allocator instrumentation does not skew timing.

use std::alloc::System;
use std::collections::BTreeMap;
use std::fs;
use std::hint::black_box;

use binostr::stats::{self, Format};
use binostr::{EventSampler, NostrEvent};
use serde::Serialize;
use sha2::{Digest, Sha256};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const DEFAULT_SEED: u64 = 0x4249_4e4f_5354_5201;
const EVENT_COUNT: usize = 1_000;

#[derive(Serialize)]
struct AllocationMetric {
    encode_allocations_per_event: f64,
    encode_allocated_bytes_per_event: f64,
    decode_allocations_per_event: f64,
    decode_allocated_bytes_per_event: f64,
}

#[derive(Serialize)]
struct AllocationReport {
    schema_version: u32,
    event_count: usize,
    seed: u64,
    corpus_id_sha256: String,
    formats: BTreeMap<&'static str, AllocationMetric>,
}

fn load_events(seed: u64) -> (Vec<NostrEvent>, String) {
    let mut sampler = EventSampler::from_directory_seeded("data", seed).expect("tracked corpus");
    let events: Vec<_> = sampler
        .random_sample(EVENT_COUNT)
        .into_iter()
        .cloned()
        .collect();
    assert_eq!(events.len(), EVENT_COUNT);
    let mut digest = Sha256::new();
    for event in &events {
        digest.update(event.id);
    }
    (events, hex::encode(digest.finalize()))
}

fn measure(events: &[NostrEvent], format: Format) -> AllocationMetric {
    // Initialize lazy schemas and build decode inputs before either region.
    let encoded: Vec<_> = events
        .iter()
        .map(|event| stats::serialize(event, format))
        .collect();
    black_box(stats::deserialize(&encoded[0], format).unwrap());

    let region = Region::new(GLOBAL);
    let outputs: Vec<_> = events
        .iter()
        .map(|event| stats::serialize(event, format))
        .collect();
    black_box(&outputs);
    let encode = region.change();
    drop(outputs);

    let region = Region::new(GLOBAL);
    let outputs: Vec<_> = encoded
        .iter()
        .map(|data| stats::deserialize(data, format).unwrap())
        .collect();
    black_box(&outputs);
    let decode = region.change();
    drop(outputs);

    let count = events.len() as f64;
    AllocationMetric {
        encode_allocations_per_event: encode.allocations as f64 / count,
        encode_allocated_bytes_per_event: (encode.bytes_allocated as isize
            + encode.bytes_reallocated) as f64
            / count,
        decode_allocations_per_event: decode.allocations as f64 / count,
        decode_allocated_bytes_per_event: (decode.bytes_allocated as isize
            + decode.bytes_reallocated) as f64
            / count,
    }
}

fn main() {
    let seed = std::env::var("BINOSTR_BENCH_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_SEED);
    let (events, fingerprint) = load_events(seed);
    let formats = Format::enabled()
        .into_iter()
        .map(|format| {
            eprintln!("  {}", format.name());
            (format.config_key(), measure(&events, format))
        })
        .collect();
    let report = AllocationReport {
        schema_version: 1,
        event_count: events.len(),
        seed,
        corpus_id_sha256: fingerprint,
        formats,
    };
    fs::create_dir_all("results").unwrap();
    fs::write(
        "results/allocations.json",
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("Wrote results/allocations.json");
}
