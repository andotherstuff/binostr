//! Comprehensive Benchmark Report
//!
//! Run with: `cargo run --release --example bench_report`
//!
//! This produces a single report comparing all formats on:
//! - Serialization speed
//! - Deserialization speed
//! - Wire size (raw and compressed)

use binostr::{capnp, cbor, dannypack, json, notepack, proto, EventLoader, NostrEvent};
use std::time::Instant;

const WARMUP_ITERATIONS: usize = 100;
const BENCH_ITERATIONS: usize = 1000;
const EVENT_COUNT: usize = 1000;

#[derive(Clone)]
struct FormatResult {
    name: &'static str,
    short_name: &'static str,
    serialize_ns: u64,
    deserialize_ns: u64,
    avg_size: usize,
    total_size: usize,
    gzip_size: usize,
    zstd_size: usize,
}

fn load_events() -> Vec<NostrEvent> {
    match EventLoader::open("data/sample.pb.gz") {
        Ok(loader) => {
            let events = loader.load_limited(EVENT_COUNT).unwrap_or_default();
            if events.is_empty() {
                eprintln!("Warning: No events loaded from data file");
            }
            events
        }
        Err(e) => {
            eprintln!("Error loading events: {}", e);
            eprintln!("Please ensure data/sample.pb.gz exists");
            std::process::exit(1);
        }
    }
}

/// Measure time for a closure, returning nanoseconds per iteration
fn bench<F: FnMut()>(mut f: F, iterations: usize) -> u64 {
    // Warmup
    for _ in 0..WARMUP_ITERATIONS {
        f();
    }

    // Measure
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();

    elapsed.as_nanos() as u64 / iterations as u64
}

fn format_ns(ns: u64) -> String {
    if ns >= 1_000_000 {
        format!("{:.2} ms", ns as f64 / 1_000_000.0)
    } else if ns >= 1_000 {
        format!("{:.2} µs", ns as f64 / 1_000.0)
    } else {
        format!("{} ns", ns)
    }
}

fn format_size(bytes: usize) -> String {
    if bytes >= 1_000_000 {
        format!("{:.2} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.2} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{} B", bytes)
    }
}

fn format_throughput(ns_per_batch: u64, event_count: usize) -> String {
    let events_per_sec = (event_count as f64 / ns_per_batch as f64) * 1_000_000_000.0;
    if events_per_sec >= 1_000_000.0 {
        format!("{:.2}M/s", events_per_sec / 1_000_000.0)
    } else if events_per_sec >= 1_000.0 {
        format!("{:.1}K/s", events_per_sec / 1_000.0)
    } else {
        format!("{:.0}/s", events_per_sec)
    }
}

fn measure_format<S, D>(
    name: &'static str,
    short_name: &'static str,
    events: &[NostrEvent],
    serialize: S,
    deserialize: D,
) -> FormatResult
where
    S: Fn(&NostrEvent) -> Vec<u8>,
    D: Fn(&[u8]) -> NostrEvent,
{
    // Pre-serialize for deserialization benchmark
    let serialized: Vec<Vec<u8>> = events.iter().map(&serialize).collect();

    // Measure serialization
    let serialize_ns = bench(
        || {
            for event in events {
                std::hint::black_box(serialize(event));
            }
        },
        BENCH_ITERATIONS,
    );

    // Measure deserialization
    let deserialize_ns = bench(
        || {
            for data in &serialized {
                std::hint::black_box(deserialize(data));
            }
        },
        BENCH_ITERATIONS,
    );

    // Calculate sizes
    let total_size: usize = serialized.iter().map(|s| s.len()).sum();
    let avg_size = total_size / events.len();

    // Concatenate all data for compression test
    let all_data: Vec<u8> = serialized.iter().flat_map(|s| s.iter().copied()).collect();
    let gzip_size = {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(6));
        encoder.write_all(&all_data).unwrap();
        encoder.finish().unwrap().len()
    };
    let zstd_size = zstd::encode_all(all_data.as_slice(), 3).unwrap().len();

    FormatResult {
        name,
        short_name,
        serialize_ns,
        deserialize_ns,
        avg_size,
        total_size,
        gzip_size,
        zstd_size,
    }
}

fn main() {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    BINOSTR COMPREHENSIVE BENCHMARK REPORT                    ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    println!();

    // Load events
    print!("Loading events... ");
    let events = load_events();
    println!("✓ {} events loaded", events.len());

    println!("Running benchmarks ({} iterations each)...", BENCH_ITERATIONS);
    println!();

    // Measure all formats
    let mut results = Vec::new();

    print!("  JSON...           ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    results.push(measure_format(
        "JSON",
        "json",
        &events,
        json::serialize,
        |d| json::deserialize(d).unwrap(),
    ));
    println!("✓");

    print!("  CBOR Schemaless... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    results.push(measure_format(
        "CBOR Schemaless",
        "cbor_schema",
        &events,
        cbor::schemaless::serialize,
        |d| cbor::schemaless::deserialize(d).unwrap(),
    ));
    println!("✓");

    print!("  CBOR Packed...    ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    results.push(measure_format(
        "CBOR Packed",
        "cbor_packed",
        &events,
        cbor::packed::serialize,
        |d| cbor::packed::deserialize(d).unwrap(),
    ));
    println!("✓");

    print!("  CBOR IntKey...    ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    results.push(measure_format(
        "CBOR IntKey",
        "cbor_intkey",
        &events,
        cbor::intkey::serialize,
        |d| cbor::intkey::deserialize(d).unwrap(),
    ));
    println!("✓");

    print!("  Proto String...   ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    results.push(measure_format(
        "Proto String",
        "proto_str",
        &events,
        proto::string::serialize,
        |d| proto::string::deserialize(d).unwrap(),
    ));
    println!("✓");

    print!("  Proto Binary...   ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    results.push(measure_format(
        "Proto Binary",
        "proto_bin",
        &events,
        proto::binary::serialize,
        |d| proto::binary::deserialize(d).unwrap(),
    ));
    println!("✓");

    print!("  Cap'n Proto...    ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    results.push(measure_format(
        "Cap'n Proto",
        "capnp",
        &events,
        capnp::serialize_event,
        |d| capnp::deserialize_event(d).unwrap(),
    ));
    println!("✓");

    print!("  Cap'n Packed...   ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    results.push(measure_format(
        "Cap'n Packed",
        "capnp_pk",
        &events,
        capnp::serialize_event_packed,
        |d| capnp::deserialize_event_packed(d).unwrap(),
    ));
    println!("✓");

    print!("  DannyPack...      ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    results.push(measure_format(
        "DannyPack",
        "dannypack",
        &events,
        |e| {
            let mut buf = Vec::new();
            dannypack::serialize(e, &mut buf);
            buf
        },
        |d| dannypack::deserialize(d).unwrap(),
    ));
    println!("✓");

    print!("  Notepack...       ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    results.push(measure_format(
        "Notepack",
        "notepack",
        &events,
        notepack::serialize,
        |d| notepack::deserialize(d).unwrap(),
    ));
    println!("✓");

    println!();

    // Find winners for highlighting
    let json_result = results.iter().find(|r| r.short_name == "json").unwrap();
    let json_size = json_result.total_size;

    let fastest_serialize = results.iter().map(|r| r.serialize_ns).min().unwrap();
    let fastest_deserialize = results.iter().map(|r| r.deserialize_ns).min().unwrap();
    let smallest_raw = results.iter().map(|r| r.total_size).min().unwrap();
    let smallest_gzip = results.iter().map(|r| r.gzip_size).min().unwrap();
    let smallest_zstd = results.iter().map(|r| r.zstd_size).min().unwrap();

    // Print comprehensive table
    // Note: Using * for winners instead of emoji to maintain alignment
    println!();
    println!("┌─────────────────┬──────────────────────────────────┬──────────────────────────────────┬──────────────────────────────────────────────┐");
    println!("│                 │          SERIALIZATION           │         DESERIALIZATION          │                     SIZE                     │");
    println!("│     FORMAT      ├────────────┬─────────────────────┼────────────┬─────────────────────┼──────────┬─────────┬──────────┬─────────────┤");
    println!("│                 │    Time    │     Throughput      │    Time    │     Throughput      │   Raw    │ vs JSON │  +gzip   │   +zstd     │");
    println!("├─────────────────┼────────────┼─────────────────────┼────────────┼─────────────────────┼──────────┼─────────┼──────────┼─────────────┤");

    for r in &results {
        let ser_best = if r.serialize_ns == fastest_serialize { "*" } else { " " };
        let deser_best = if r.deserialize_ns == fastest_deserialize { "*" } else { " " };
        let raw_best = if r.total_size == smallest_raw { "*" } else { " " };
        let gzip_best = if r.gzip_size == smallest_gzip { "*" } else { " " };
        let zstd_best = if r.zstd_size == smallest_zstd { "*" } else { " " };

        let size_vs_json = 100.0 * r.total_size as f64 / json_size as f64;

        println!(
            "│ {:<15} │ {:>9}{} │ {:>18} │ {:>9}{} │ {:>18} │ {:>7}{} │ {:>6.1}% │ {:>7}{} │ {:>10}{} │",
            r.name,
            format_ns(r.serialize_ns),
            ser_best,
            format_throughput(r.serialize_ns, events.len()),
            format_ns(r.deserialize_ns),
            deser_best,
            format_throughput(r.deserialize_ns, events.len()),
            format_size(r.avg_size),
            raw_best,
            size_vs_json,
            format_size(r.gzip_size),
            gzip_best,
            format_size(r.zstd_size),
            zstd_best,
        );
    }

    println!("└─────────────────┴────────────┴─────────────────────┴────────────┴─────────────────────┴──────────┴─────────┴──────────┴─────────────┘");
    println!();
    println!("  * = best in category");
    println!();

    // Print rankings
    println!("┌──────────────────────────────────────────────────────────────────────────────┐");
    println!("│                              RANKINGS BY METRIC                              │");
    println!("└──────────────────────────────────────────────────────────────────────────────┘");
    println!();

    // Sort and print serialization ranking
    let mut ser_sorted = results.clone();
    ser_sorted.sort_by_key(|r| r.serialize_ns);
    println!("  📝 SERIALIZATION SPEED (fastest first):");
    for (i, r) in ser_sorted.iter().enumerate() {
        let speedup = json_result.serialize_ns as f64 / r.serialize_ns as f64;
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        println!(
            "     {} {:2}. {:<15} {:>10} ({:>15}) {:.1}x vs JSON",
            medal,
            i + 1,
            r.name,
            format_ns(r.serialize_ns),
            format_throughput(r.serialize_ns, events.len()),
            speedup
        );
    }
    println!();

    // Sort and print deserialization ranking
    let mut deser_sorted = results.clone();
    deser_sorted.sort_by_key(|r| r.deserialize_ns);
    println!("  📖 DESERIALIZATION SPEED (fastest first):");
    for (i, r) in deser_sorted.iter().enumerate() {
        let speedup = json_result.deserialize_ns as f64 / r.deserialize_ns as f64;
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        println!(
            "     {} {:2}. {:<15} {:>10} ({:>15}) {:.1}x vs JSON",
            medal,
            i + 1,
            r.name,
            format_ns(r.deserialize_ns),
            format_throughput(r.deserialize_ns, events.len()),
            speedup
        );
    }
    println!();

    // Sort and print size ranking
    let mut size_sorted = results.clone();
    size_sorted.sort_by_key(|r| r.total_size);
    println!("  📦 RAW SIZE (smallest first):");
    for (i, r) in size_sorted.iter().enumerate() {
        let pct = 100.0 * r.total_size as f64 / json_size as f64;
        let savings = 100.0 - pct;
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        println!(
            "     {} {:2}. {:<15} {:>10} ({:>5.1}% of JSON, {:>5.1}% savings)",
            medal,
            i + 1,
            r.name,
            format_size(r.avg_size),
            pct,
            savings
        );
    }
    println!();

    // Sort and print compressed size ranking
    let mut zstd_sorted = results.clone();
    zstd_sorted.sort_by_key(|r| r.zstd_size);
    println!("  🗜️  COMPRESSED SIZE (zstd, smallest first):");
    let json_zstd = json_result.zstd_size;
    for (i, r) in zstd_sorted.iter().enumerate() {
        let pct = 100.0 * r.zstd_size as f64 / json_zstd as f64;
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        println!(
            "     {} {:2}. {:<15} {:>10} ({:>5.1}% of JSON compressed)",
            medal,
            i + 1,
            r.name,
            format_size(r.zstd_size),
            pct
        );
    }
    println!();

    // Print summary recommendation
    println!("┌──────────────────────────────────────────────────────────────────────────────┐");
    println!("│                               RECOMMENDATIONS                                │");
    println!("└──────────────────────────────────────────────────────────────────────────────┘");
    println!();

    // Find best overall (weighted score)
    let best_speed = ser_sorted[0].name;
    let best_size = size_sorted[0].name;
    let best_deser = deser_sorted[0].name;

    println!("  • Fastest serialization:    {}", best_speed);
    println!("  • Fastest deserialization:  {}", best_deser);
    println!("  • Smallest wire size:       {}", best_size);
    println!();

    // Calculate a balanced score (normalize each metric, lower is better)
    let mut balanced: Vec<(&str, f64)> = results
        .iter()
        .map(|r| {
            let ser_score = r.serialize_ns as f64 / fastest_serialize as f64;
            let deser_score = r.deserialize_ns as f64 / fastest_deserialize as f64;
            let size_score = r.total_size as f64 / smallest_raw as f64;
            // Weight: 30% serialize, 30% deserialize, 40% size
            let total = 0.3 * ser_score + 0.3 * deser_score + 0.4 * size_score;
            (r.name, total)
        })
        .collect();
    balanced.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("  🎯 BALANCED RECOMMENDATION (30% ser + 30% deser + 40% size):");
    for (i, (name, score)) in balanced.iter().take(3).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        println!("     {} {:<15} (score: {:.2})", medal, name, score);
    }
    println!();

    println!("  📋 For a Nostr NIP recommendation:");
    println!("     • Best for bandwidth-constrained: {}", best_size);
    println!("     • Best for CPU-constrained:       {}", best_speed);
    println!("     • Best balanced:                  {}", balanced[0].0);
    println!();
}
