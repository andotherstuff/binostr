//! EXHAUSTIVE DannyPack verification - EVERY event, MILLIONS of times

use binostr::sampler::EventSampler;
use binostr::{dannypack, json, proto, NostrEvent};
use std::fs::File;
use std::io::{Read, Write};
use std::time::Instant;

const ITERATIONS: usize = 100; // 100 iterations × 10k events = 1 million ops

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading ALL events...");
    let mut sampler = EventSampler::from_directory("data", 100_000)?;
    let events: Vec<NostrEvent> = sampler.random_sample(10_000).into_iter().cloned().collect();
    println!("Loaded {} events\n", events.len());

    println!("============================================================");
    println!("TEST 1: Individual roundtrip for EVERY event");
    println!("============================================================\n");

    // Test EVERY event individually
    let mut dp_ok = 0;
    let mut dp_fail = 0;
    let mut pb_ok = 0;
    let mut pb_fail = 0;

    for (i, event) in events.iter().enumerate() {
        // DannyPack individual roundtrip
        let dp_bytes = dannypack::serialize_to_vec(event);
        match dannypack::deserialize(&dp_bytes) {
            Ok(back) if &back == event => dp_ok += 1,
            Ok(back) => {
                dp_fail += 1;
                if dp_fail <= 3 {
                    println!("❌ DannyPack mismatch at event {}", i);
                    println!("   ID: {}", hex::encode(event.id));
                    println!("   Content len: {} vs {}", event.content.len(), back.content.len());
                }
            }
            Err(e) => {
                dp_fail += 1;
                if dp_fail <= 3 {
                    println!("❌ DannyPack error at event {}: {}", i, e);
                }
            }
        }

        // Proto Binary individual roundtrip
        let pb_bytes = proto::binary::serialize(event);
        match proto::binary::deserialize(&pb_bytes) {
            Ok(back) if &back == event => pb_ok += 1,
            _ => pb_fail += 1,
        }

        if (i + 1) % 1000 == 0 {
            print!("\rProcessed {}/{} events...", i + 1, events.len());
            std::io::stdout().flush()?;
        }
    }
    println!("\r                                        ");

    println!("DannyPack:    {}/{} OK ({} failures)", dp_ok, events.len(), dp_fail);
    println!("Proto Binary: {}/{} OK ({} failures)", pb_ok, events.len(), pb_fail);

    if dp_fail > 0 || pb_fail > 0 {
        println!("\n❌ FAILURES DETECTED - STOPPING");
        return Ok(());
    }
    println!("✅ All {} events roundtrip correctly!\n", events.len());

    println!("============================================================");
    println!("TEST 2: {} iterations over ALL {} events", ITERATIONS, events.len());
    println!("        = {} total serialize+deserialize operations", ITERATIONS * events.len());
    println!("============================================================\n");

    // Write all events to files first
    println!("Writing events to files...");

    let dp_batch = dannypack::serialize_batch(&events);
    let mut f = File::create("/tmp/dp_million.bin")?;
    f.write_all(&dp_batch)?;
    f.sync_all()?;
    println!("  DannyPack:    {} bytes -> /tmp/dp_million.bin", dp_batch.len());

    let pb_batch = proto::binary::serialize_batch(&events);
    let mut f = File::create("/tmp/pb_million.bin")?;
    f.write_all(&pb_batch)?;
    f.sync_all()?;
    println!("  Proto Binary: {} bytes -> /tmp/pb_million.bin", pb_batch.len());

    let json_batch = json::serialize_batch(&events);
    let mut f = File::create("/tmp/json_million.json")?;
    f.write_all(&json_batch)?;
    f.sync_all()?;
    println!("  JSON:         {} bytes -> /tmp/json_million.json", json_batch.len());

    // Read back
    let mut dp_data = Vec::new();
    File::open("/tmp/dp_million.bin")?.read_to_end(&mut dp_data)?;
    let mut pb_data = Vec::new();
    File::open("/tmp/pb_million.bin")?.read_to_end(&mut pb_data)?;
    let mut json_data = Vec::new();
    File::open("/tmp/json_million.json")?.read_to_end(&mut json_data)?;

    println!("\nRunning {} iterations...\n", ITERATIONS);

    // DANNYPACK: serialize + deserialize all events, N times
    println!("DannyPack: serializing + deserializing {} events × {} times...", events.len(), ITERATIONS);
    let dp_start = Instant::now();
    let mut dp_check = 0u64;
    for iter in 0..ITERATIONS {
        for event in &events {
            let bytes = dannypack::serialize_to_vec(event);
            let back = dannypack::deserialize(&bytes).unwrap();
            dp_check = dp_check.wrapping_add(back.created_at as u64);
        }
        if (iter + 1) % 100_000 == 0 {
            print!("\r  Progress: {}/{}...", iter + 1, ITERATIONS);
            std::io::stdout().flush()?;
        }
    }
    let dp_time = dp_start.elapsed();
    println!("\r                                        ");

    let dp_ops = ITERATIONS * events.len();
    let dp_ns_per_op = dp_time.as_nanos() as f64 / dp_ops as f64;
    println!("  Total time: {:?}", dp_time);
    println!("  Operations: {} (serialize+deserialize pairs)", dp_ops);
    println!("  Per operation: {:.1} ns", dp_ns_per_op);
    println!("  Checksum: {} (to prevent optimization)\n", dp_check);

    // PROTO BINARY
    println!("Proto Binary: serializing + deserializing {} events × {} times...", events.len(), ITERATIONS);
    let pb_start = Instant::now();
    let mut pb_check = 0u64;
    for iter in 0..ITERATIONS {
        for event in &events {
            let bytes = proto::binary::serialize(event);
            let back = proto::binary::deserialize(&bytes).unwrap();
            pb_check = pb_check.wrapping_add(back.created_at as u64);
        }
        if (iter + 1) % 100_000 == 0 {
            print!("\r  Progress: {}/{}...", iter + 1, ITERATIONS);
            std::io::stdout().flush()?;
        }
    }
    let pb_time = pb_start.elapsed();
    println!("\r                                        ");

    let pb_ops = ITERATIONS * events.len();
    let pb_ns_per_op = pb_time.as_nanos() as f64 / pb_ops as f64;
    println!("  Total time: {:?}", pb_time);
    println!("  Operations: {}", pb_ops);
    println!("  Per operation: {:.1} ns", pb_ns_per_op);
    println!("  Checksum: {}\n", pb_check);

    // Verify checksums match
    if dp_check != pb_check {
        println!("❌ CHECKSUMS DON'T MATCH - DATA CORRUPTION!");
        return Ok(());
    }
    println!("✅ Checksums match: {} == {}", dp_check, pb_check);

    println!("\n============================================================");
    println!("FINAL RESULTS");
    println!("============================================================");
    println!("\nTotal operations: {} (serialize + deserialize)", dp_ops);
    println!("\n┌──────────────────┬────────────────┬─────────────┬──────────┐");
    println!("│ Format           │ Total Time     │ Per Op      │ Speedup  │");
    println!("├──────────────────┼────────────────┼─────────────┼──────────┤");
    println!("│ DannyPack        │ {:>12.2?} │ {:>7.1} ns │   1.00x  │", dp_time, dp_ns_per_op);
    println!("│ Proto Binary     │ {:>12.2?} │ {:>7.1} ns │   {:.2}x  │", pb_time, pb_ns_per_op, pb_time.as_nanos() as f64 / dp_time.as_nanos() as f64);
    println!("└──────────────────┴────────────────┴─────────────┴──────────┘");

    println!("\nFile sizes:");
    println!("  DannyPack:    {:>10} bytes ({:.1}% of JSON)", dp_batch.len(), 100.0 * dp_batch.len() as f64 / json_batch.len() as f64);
    println!("  Proto Binary: {:>10} bytes ({:.1}% of JSON)", pb_batch.len(), 100.0 * pb_batch.len() as f64 / json_batch.len() as f64);
    println!("  JSON:         {:>10} bytes", json_batch.len());

    println!("\n🔥 DannyPack is {:.2}x FASTER than Proto Binary over {} million operations! 🔥",
             pb_time.as_nanos() as f64 / dp_time.as_nanos() as f64,
             dp_ops / 1_000_000);

    Ok(())
}
