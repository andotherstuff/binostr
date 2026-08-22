//! Generate the machine-readable and human-readable publication snapshot.
//!
//! Run with `cargo run --release --example public_report`.

use std::collections::BTreeMap;
use std::fs;
use std::hint::black_box;
use std::process::Command;
use std::time::Instant;

use binostr::stats::{self, Format, FormatClass};
use binostr::validation::{self, EventLimits};
use binostr::{EventSampler, NostrEvent};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DEFAULT_SEED: u64 = 0x4249_4e4f_5354_5201;
const EVENT_COUNT: usize = 1_000;

#[derive(Serialize)]
struct Metadata {
    generated_utc: String,
    git_commit: String,
    git_dirty: bool,
    rustc: String,
    host: String,
    profile: &'static str,
    event_count: usize,
    seed: u64,
    corpus_id_sha256: String,
    iterations: usize,
    latency_chunk_events: usize,
}

#[derive(Serialize)]
struct Metric {
    format: &'static str,
    key: &'static str,
    class: &'static str,
    encode_ns_per_event: f64,
    decode_ns_per_event: f64,
    decode_id_ns_per_event: f64,
    validated_ns_per_event: f64,
    encode_p50_ns: u64,
    encode_p95_ns: u64,
    encode_p99_ns: u64,
    decode_p50_ns: u64,
    decode_p95_ns: u64,
    decode_p99_ns: u64,
    encode_allocations_per_event: f64,
    encode_allocated_bytes_per_event: f64,
    decode_allocations_per_event: f64,
    decode_allocated_bytes_per_event: f64,
    average_wire_bytes: f64,
    framed_stream_bytes: usize,
    framed_zstd_bytes: usize,
    framed_gzip_bytes: usize,
    native_batch_bytes: usize,
    native_batch_encode_ns_per_event: f64,
    native_batch_decode_ns_per_event: f64,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    metadata: Metadata,
    metrics: Vec<Metric>,
}

#[derive(Deserialize)]
struct AllocationMetric {
    encode_allocations_per_event: f64,
    encode_allocated_bytes_per_event: f64,
    decode_allocations_per_event: f64,
    decode_allocated_bytes_per_event: f64,
}

#[derive(Deserialize)]
struct AllocationReport {
    event_count: usize,
    seed: u64,
    corpus_id_sha256: String,
    formats: BTreeMap<String, AllocationMetric>,
}

fn command(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn class_name(class: FormatClass) -> &'static str {
    match class {
        FormatClass::NostrBaseline => "nostr-baseline",
        FormatClass::InteroperableCandidate => "interoperable-candidate",
        FormatClass::EmergingReference => "emerging-reference",
        FormatClass::StandardEnvelopeCustomProfile => "standard-envelope-custom-profile",
        FormatClass::RustReference => "rust-reference",
        FormatClass::CustomReference => "custom-reference",
        FormatClass::LegacyProfile => "legacy-profile",
    }
}

fn load_events(seed: u64) -> (Vec<NostrEvent>, String) {
    let mut sampler = EventSampler::from_directory_seeded("data", seed).expect("tracked corpus");
    let events: Vec<_> = sampler
        .random_sample(EVENT_COUNT)
        .into_iter()
        .cloned()
        .collect();
    assert_eq!(
        events.len(),
        EVENT_COUNT,
        "publication report requires {EVENT_COUNT} events"
    );
    let mut digest = Sha256::new();
    for event in &events {
        digest.update(event.id);
    }
    (events, hex::encode(digest.finalize()))
}

fn average_time(iterations: usize, mut operation: impl FnMut()) -> u128 {
    for _ in 0..3 {
        operation();
    }
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    start.elapsed().as_nanos() / iterations as u128
}

fn percentiles(mut values: Vec<u64>) -> (u64, u64, u64) {
    values.sort_unstable();
    let at = |percent: usize| values[((values.len() - 1) * percent) / 100];
    (at(50), at(95), at(99))
}

fn latency(
    events: &[NostrEvent],
    encoded: &[Vec<u8>],
    format: Format,
    chunk: usize,
) -> ((u64, u64, u64), (u64, u64, u64)) {
    let mut encode = Vec::new();
    let mut decode = Vec::new();
    for _ in 0..5 {
        for values in events.chunks(chunk) {
            let start = Instant::now();
            for event in values {
                black_box(stats::serialize(event, format));
            }
            encode.push((start.elapsed().as_nanos() / values.len() as u128) as u64);
        }
        for values in encoded.chunks(chunk) {
            let start = Instant::now();
            for data in values {
                black_box(stats::deserialize(data, format).unwrap());
            }
            decode.push((start.elapsed().as_nanos() / values.len() as u128) as u64);
        }
    }
    (percentiles(encode), percentiles(decode))
}

fn framed(records: &[Vec<u8>]) -> Vec<u8> {
    let mut output = Vec::with_capacity(records.iter().map(|record| 4 + record.len()).sum());
    for record in records {
        output.extend_from_slice(&u32::try_from(record.len()).unwrap().to_be_bytes());
        output.extend_from_slice(record);
    }
    output
}

fn measure(
    events: &[NostrEvent],
    format: Format,
    iterations: usize,
    allocation: &AllocationMetric,
) -> Metric {
    let encoded: Vec<_> = events
        .iter()
        .map(|event| stats::serialize(event, format))
        .collect();
    for (event, data) in events.iter().zip(&encoded) {
        assert_eq!(*event, stats::deserialize(data, format).unwrap());
    }

    let encode_ns = average_time(iterations, || {
        for event in events {
            black_box(stats::serialize(event, format));
        }
    });
    let decode_ns = average_time(iterations, || {
        for data in &encoded {
            black_box(stats::deserialize(data, format).unwrap());
        }
    });
    let decode_id_ns = average_time(iterations, || {
        for data in &encoded {
            let event = stats::deserialize(data, format).unwrap();
            validation::verify_id(&event).unwrap();
            black_box(&event);
        }
    });
    let limits = EventLimits::default();
    let validated_ns = average_time(iterations, || {
        for data in &encoded {
            let event = stats::deserialize_limited(data, format, &limits).unwrap();
            validation::verify_id_and_signature(&event).unwrap();
            black_box(&event);
        }
    });

    let chunk = 20;
    let (encode_tail, decode_tail) = latency(events, &encoded, format, chunk);
    let stream = framed(&encoded);
    let native_batch = stats::serialize_batch(events, format);
    assert_eq!(
        events,
        stats::deserialize_batch(&native_batch, format).unwrap()
    );
    let batch_encode_ns = average_time(iterations, || {
        black_box(stats::serialize_batch(events, format));
    });
    let batch_decode_ns = average_time(iterations, || {
        black_box(stats::deserialize_batch(&native_batch, format).unwrap());
    });

    Metric {
        format: format.name(),
        key: format.config_key(),
        class: class_name(format.class()),
        encode_ns_per_event: encode_ns as f64 / events.len() as f64,
        decode_ns_per_event: decode_ns as f64 / events.len() as f64,
        decode_id_ns_per_event: decode_id_ns as f64 / events.len() as f64,
        validated_ns_per_event: validated_ns as f64 / events.len() as f64,
        encode_p50_ns: encode_tail.0,
        encode_p95_ns: encode_tail.1,
        encode_p99_ns: encode_tail.2,
        decode_p50_ns: decode_tail.0,
        decode_p95_ns: decode_tail.1,
        decode_p99_ns: decode_tail.2,
        encode_allocations_per_event: allocation.encode_allocations_per_event,
        encode_allocated_bytes_per_event: allocation.encode_allocated_bytes_per_event,
        decode_allocations_per_event: allocation.decode_allocations_per_event,
        decode_allocated_bytes_per_event: allocation.decode_allocated_bytes_per_event,
        average_wire_bytes: encoded.iter().map(Vec::len).sum::<usize>() as f64
            / events.len() as f64,
        framed_stream_bytes: stream.len(),
        framed_zstd_bytes: zstd::encode_all(stream.as_slice(), 3).unwrap().len(),
        framed_gzip_bytes: {
            use flate2::write::GzEncoder;
            use flate2::Compression;
            use std::io::Write;
            let mut encoder = GzEncoder::new(Vec::new(), Compression::new(6));
            encoder.write_all(&stream).unwrap();
            encoder.finish().unwrap().len()
        },
        native_batch_bytes: native_batch.len(),
        native_batch_encode_ns_per_event: batch_encode_ns as f64 / events.len() as f64,
        native_batch_decode_ns_per_event: batch_decode_ns as f64 / events.len() as f64,
    }
}

fn markdown(report: &Report) -> String {
    let mut output = format!(
        "# Binostr benchmark snapshot\n\nGenerated `{}` from commit `{}`{} using `{}`. Corpus: {} events, seed `{}`, ID fingerprint `{}`.\n\nTimes are nanoseconds per event. Tail latency uses chunks of {} events to reduce timer noise. Allocation byte counts include reallocations.\n\n| Format | Class | Encode | Decode | Decode + ID | Fully validated | Bytes | Zstd stream | Decode allocs | Decode allocated bytes | Decode p99 |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
        report.metadata.generated_utc,
        report.metadata.git_commit,
        if report.metadata.git_dirty { " (dirty tree)" } else { "" },
        report.metadata.rustc,
        report.metadata.event_count,
        report.metadata.seed,
        report.metadata.corpus_id_sha256,
        report.metadata.latency_chunk_events,
    );
    for metric in &report.metrics {
        output.push_str(&format!(
            "| {} | {} | {:.0} | {:.0} | {:.0} | {:.0} | {:.1} | {} | {:.1} | {:.0} | {} |\n",
            metric.format,
            metric.class,
            metric.encode_ns_per_event,
            metric.decode_ns_per_event,
            metric.decode_id_ns_per_event,
            metric.validated_ns_per_event,
            metric.average_wire_bytes,
            metric.framed_zstd_bytes,
            metric.decode_allocations_per_event,
            metric.decode_allocated_bytes_per_event,
            metric.decode_p99_ns,
        ));
    }
    output.push_str("\n## Interpretation guardrails\n\n- Results compare these Rust implementations and profiles, not abstract format ceilings.\n- `Fully validated` includes bounded decode, structural checks, NIP-01 ID recomputation, and BIP-340 verification.\n- Compressed streams use the same four-byte big-endian record framing for every format. Native batch sizes and timings are retained in the JSON artifact.\n- Interoperability and ecosystem support are protocol-selection criteria that timings cannot capture.\n");
    output
}

fn main() {
    let seed = std::env::var("BINOSTR_BENCH_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_SEED);
    let iterations = std::env::var("BINOSTR_REPORT_ITERATIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(20);
    let (events, fingerprint) = load_events(seed);
    let allocations: AllocationReport = serde_json::from_slice(
        &fs::read("results/allocations.json")
            .expect("run `cargo run --release --example allocation_report` before public_report"),
    )
    .expect("valid results/allocations.json");
    assert_eq!(allocations.event_count, events.len());
    assert_eq!(allocations.seed, seed);
    assert_eq!(allocations.corpus_id_sha256, fingerprint);
    let formats = Format::enabled();
    eprintln!(
        "Measuring {} formats over {} events",
        formats.len(),
        events.len()
    );
    let metrics = formats
        .into_iter()
        .map(|format| {
            eprintln!("  {}", format.name());
            let allocation = allocations
                .formats
                .get(format.config_key())
                .unwrap_or_else(|| panic!("missing allocation data for {}", format.name()));
            measure(&events, format, iterations, allocation)
        })
        .collect();
    let git_commit = command("git", &["rev-parse", "HEAD"]);
    let report = Report {
        schema_version: 1,
        metadata: Metadata {
            generated_utc: command("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"]),
            git_commit,
            git_dirty: !command("git", &["status", "--porcelain"]).is_empty(),
            rustc: command("rustc", &["--version"]),
            host: format!(
                "{}; {}",
                command("uname", &["-a"]),
                command("sysctl", &["-n", "machdep.cpu.brand_string"])
            ),
            profile: "release",
            event_count: events.len(),
            seed,
            corpus_id_sha256: fingerprint,
            iterations,
            latency_chunk_events: 20,
        },
        metrics,
    };
    fs::create_dir_all("results").unwrap();
    fs::write(
        "results/latest.json",
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    fs::write("results/latest.md", markdown(&report)).unwrap();
    println!("Wrote results/latest.json and results/latest.md");
}
