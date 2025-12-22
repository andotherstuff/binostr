//! Comprehensive Benchmark Report
//!
//! Run with: `cargo run --release --example bench_report`
//!
//! This produces a single report comparing all formats on:
//! - Serialization speed (client-side, less critical)
//! - Deserialization speed (relay-side, critical)
//! - Wire size (storage + bandwidth, critical)
//!
//! Weightings reflect real-world Nostr relay workloads where:
//! - Relays must parse/validate every incoming event (deserialization-heavy)
//! - Storage and bandwidth costs dominate (size-critical)
//! - Clients serialize when posting (less frequent, can be slower)

use binostr::stats::Format;
use binostr::{capnp, cbor, config, dannypack, json, notepack, proto, EventLoader, NostrEvent};
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

/// Check if a format is enabled in config
fn is_enabled(short_name: &str) -> bool {
    config::is_format_enabled(short_name)
}

fn main() {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    BINOSTR COMPREHENSIVE BENCHMARK REPORT                    ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    println!();

    // Show enabled formats
    let enabled: Vec<_> = Format::enabled().iter().map(|f| f.name()).collect();
    println!("Enabled formats: {}", enabled.join(", "));
    println!();

    // Load events
    print!("Loading events... ");
    let events = load_events();
    println!("✓ {} events loaded", events.len());

    println!("Running benchmarks ({} iterations each)...", BENCH_ITERATIONS);
    println!();

    // Measure all enabled formats
    let mut results = Vec::new();

    // JSON is always included as baseline
    if is_enabled("json") {
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
    }

    if is_enabled("cbor_schemaless") {
        print!("  CBOR Schemaless... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        results.push(measure_format(
            "CBOR Schemaless",
            "cbor_schemaless",
            &events,
            cbor::schemaless::serialize,
            |d| cbor::schemaless::deserialize(d).unwrap(),
        ));
        println!("✓");
    }

    if is_enabled("cbor_packed") {
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
    }

    if is_enabled("cbor_intkey") {
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
    }

    if is_enabled("proto_string") {
        print!("  Proto String...   ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        results.push(measure_format(
            "Proto String",
            "proto_string",
            &events,
            proto::string::serialize,
            |d| proto::string::deserialize(d).unwrap(),
        ));
        println!("✓");
    }

    if is_enabled("proto_binary") {
        print!("  Proto Binary...   ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        results.push(measure_format(
            "Proto Binary",
            "proto_binary",
            &events,
            proto::binary::serialize,
            |d| proto::binary::deserialize(d).unwrap(),
        ));
        println!("✓");
    }

    if is_enabled("capnp") {
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
    }

    if is_enabled("capnp_packed") {
        print!("  Cap'n Packed...   ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        results.push(measure_format(
            "Cap'n Packed",
            "capnp_packed",
            &events,
            capnp::serialize_event_packed,
            |d| capnp::deserialize_event_packed(d).unwrap(),
        ));
        println!("✓");
    }

    if is_enabled("dannypack") {
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
    }

    if is_enabled("notepack") {
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
    }

    if results.is_empty() {
        println!("No formats enabled! Check binostr.toml");
        return;
    }

    println!();

    // Find winners for highlighting
    let json_result = results.iter().find(|r| r.short_name == "json");
    let json_size = json_result.map(|r| r.total_size).unwrap_or(results[0].total_size);
    let json_zstd = json_result.map(|r| r.zstd_size).unwrap_or(results[0].zstd_size);

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
    let json_ser_ns = json_result.map(|r| r.serialize_ns).unwrap_or(ser_sorted[0].serialize_ns);
    for (i, r) in ser_sorted.iter().enumerate() {
        let speedup = json_ser_ns as f64 / r.serialize_ns as f64;
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
    let json_deser_ns = json_result.map(|r| r.deserialize_ns).unwrap_or(deser_sorted[0].deserialize_ns);
    for (i, r) in deser_sorted.iter().enumerate() {
        let speedup = json_deser_ns as f64 / r.deserialize_ns as f64;
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
    let best_compressed = zstd_sorted[0].name;

    println!("  Category Winners:");
    println!("  • Fastest serialization:    {}", best_speed);
    println!("  • Fastest deserialization:  {}", best_deser);
    println!("  • Smallest wire size:       {}", best_size);
    println!("  • Smallest compressed:      {}", best_compressed);
    println!();

    // Calculate relay-focused score (real-world Nostr workload)
    // Rationale:
    // - 10% serialization: Clients serialize when posting, less frequent
    // - 45% deserialization: Relays must parse EVERY incoming event
    // - 30% raw size: Storage costs, memory pressure
    // - 15% compressed size: Bandwidth for transfer (often compressed)
    let mut relay_score: Vec<(&str, f64)> = results
        .iter()
        .map(|r| {
            let ser_score = r.serialize_ns as f64 / fastest_serialize as f64;
            let deser_score = r.deserialize_ns as f64 / fastest_deserialize as f64;
            let size_score = r.total_size as f64 / smallest_raw as f64;
            let zstd_score = r.zstd_size as f64 / smallest_zstd as f64;

            // Relay-focused: heavy on deserialization and size
            let total = 0.10 * ser_score
                      + 0.45 * deser_score
                      + 0.30 * size_score
                      + 0.15 * zstd_score;
            (r.name, total)
        })
        .collect();
    relay_score.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("  🖥️  RELAY-OPTIMIZED (10% ser + 45% deser + 30% raw + 15% compressed):");
    println!("     Optimized for relay workloads: fast parsing, compact storage");
    for (i, (name, score)) in relay_score.iter().take(5).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        println!("     {} {:2}. {:<15} (score: {:.3})", medal, i + 1, name, score);
    }
    println!();

    // Calculate client-focused score (prioritizes encode speed)
    // - 35% serialization: Clients need responsive posting
    // - 25% deserialization: Reading feed
    // - 25% raw size: Bandwidth usage
    // - 15% compressed size: Transfer efficiency
    let mut client_score: Vec<(&str, f64)> = results
        .iter()
        .map(|r| {
            let ser_score = r.serialize_ns as f64 / fastest_serialize as f64;
            let deser_score = r.deserialize_ns as f64 / fastest_deserialize as f64;
            let size_score = r.total_size as f64 / smallest_raw as f64;
            let zstd_score = r.zstd_size as f64 / smallest_zstd as f64;

            let total = 0.35 * ser_score
                      + 0.25 * deser_score
                      + 0.25 * size_score
                      + 0.15 * zstd_score;
            (r.name, total)
        })
        .collect();
    client_score.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("  📱 CLIENT-OPTIMIZED (35% ser + 25% deser + 25% raw + 15% compressed):");
    println!("     Optimized for client apps: responsive posting, efficient reading");
    for (i, (name, score)) in client_score.iter().take(5).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        println!("     {} {:2}. {:<15} (score: {:.3})", medal, i + 1, name, score);
    }
    println!();

    // Calculate pure size score (bandwidth/storage focused)
    // - 0% serialization
    // - 0% deserialization
    // - 60% raw size: Direct storage impact
    // - 40% compressed size: Transfer efficiency
    let mut size_score_vec: Vec<(&str, f64)> = results
        .iter()
        .map(|r| {
            let size_score = r.total_size as f64 / smallest_raw as f64;
            let zstd_score = r.zstd_size as f64 / smallest_zstd as f64;
            let total = 0.60 * size_score + 0.40 * zstd_score;
            (r.name, total)
        })
        .collect();
    size_score_vec.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("  💾 SIZE-OPTIMIZED (60% raw + 40% compressed):");
    println!("     Optimized for storage/bandwidth: smallest footprint");
    for (i, (name, score)) in size_score_vec.iter().take(5).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        println!("     {} {:2}. {:<15} (score: {:.3})", medal, i + 1, name, score);
    }
    println!();

    println!("┌──────────────────────────────────────────────────────────────────────────────┐");
    println!("│                          NOSTR NIP RECOMMENDATION                            │");
    println!("└──────────────────────────────────────────────────────────────────────────────┘");
    println!();
    println!("  For a binary Nostr format NIP, the primary concerns are:");
    println!("  1. Relay performance (parsing millions of events/day)");
    println!("  2. Storage efficiency (disk costs scale with data)");
    println!("  3. Bandwidth savings (users on mobile/metered connections)");
    println!();
    println!("  Based on relay-optimized scoring:");
    println!("     🏆 RECOMMENDED: {}", relay_score[0].0);
    if relay_score.len() > 1 {
        println!("     🥈 Runner-up:   {}", relay_score[1].0);
    }
    println!();
    println!("  Key metrics for {}:", relay_score[0].0);
    if let Some(winner) = results.iter().find(|r| r.name == relay_score[0].0) {
        let size_vs_json = 100.0 * winner.total_size as f64 / json_size as f64;
        let zstd_vs_json = 100.0 * winner.zstd_size as f64 / json_zstd as f64;
        println!("     • Raw size:    {:.1}% of JSON ({:.1}% savings)", size_vs_json, 100.0 - size_vs_json);
        println!("     • Compressed:  {:.1}% of JSON+zstd", zstd_vs_json);
        println!("     • Decode:      {} ({})",
            format_ns(winner.deserialize_ns),
            format_throughput(winner.deserialize_ns, events.len()));
        println!("     • Encode:      {} ({})",
            format_ns(winner.serialize_ns),
            format_throughput(winner.serialize_ns, events.len()));
    }
    println!();
}
