//! Batch overhead analysis
//!
//! Analyzes the overhead of batching events vs individual serialization
//! to help understand when batching is beneficial.

use binostr::stats::Format;
use binostr::{EventLoader, NostrEvent};

fn load_events(count: usize) -> Vec<NostrEvent> {
    match EventLoader::open("data/sample.pb.gz") {
        Ok(loader) => loader.load_limited(count).unwrap_or_default(),
        Err(e) => {
            eprintln!("Could not load events: {}", e);
            Vec::new()
        }
    }
}

fn main() {
    let events = load_events(1000);
    if events.is_empty() {
        eprintln!("No events loaded");
        return;
    }

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              BATCH OVERHEAD ANALYSIS                         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Events loaded: {}\n", events.len());

    // Analyze batch sizes
    for batch_size in [10, 100, 1000] {
        if batch_size > events.len() {
            continue;
        }

        let batch = &events[..batch_size];

        println!("═══ Batch size: {} events ═══\n", batch_size);
        println!(
            "{:<20} {:>12} {:>12} {:>10}",
            "Format", "Individual", "Batched", "Overhead"
        );
        println!("{}", "─".repeat(56));

        for format in Format::enabled() {
            // Individual serialization total
            let individual_total: usize = batch
                .iter()
                .map(|e| binostr::stats::serialize(e, format).len())
                .sum();

            // Batch serialization
            let batch_total = binostr::stats::serialize_batch(batch, format).len();

            // Calculate overhead
            let overhead = batch_total as f64 / individual_total as f64;
            let overhead_pct = (overhead - 1.0) * 100.0;

            let overhead_str = if overhead_pct >= 0.0 {
                format!("+{:.1}%", overhead_pct)
            } else {
                format!("{:.1}%", overhead_pct)
            };

            println!(
                "{:<20} {:>12} {:>12} {:>10}",
                format.name(),
                individual_total,
                batch_total,
                overhead_str
            );
        }
        println!();
    }

    // Per-event overhead breakdown
    println!("═══ Per-Event Batch Wrapper Overhead ═══\n");
    println!("(Difference between batch and sum of individual sizes)\n");

    let batch = &events[..100];
    println!(
        "{:<20} {:>15} {:>15}",
        "Format", "Total Overhead", "Per Event"
    );
    println!("{}", "─".repeat(52));

    for format in Format::enabled() {
        let individual_total: usize = batch
            .iter()
            .map(|e| binostr::stats::serialize(e, format).len())
            .sum();
        let batch_total = binostr::stats::serialize_batch(batch, format).len();

        let total_overhead = batch_total as i64 - individual_total as i64;
        let per_event = total_overhead as f64 / batch.len() as f64;

        println!(
            "{:<20} {:>15} {:>15.1}",
            format.name(),
            total_overhead,
            per_event
        );
    }

    println!("\n═══ Compression Impact on Batching ═══\n");
    println!("(Zstd level 3 compression)\n");

    let batch = &events[..100];
    println!(
        "{:<20} {:>10} {:>10} {:>10} {:>10}",
        "Format", "Indiv Raw", "Batch Raw", "Indiv Zstd", "Batch Zstd"
    );
    println!("{}", "─".repeat(62));

    for format in Format::enabled() {
        let individual_total: usize = batch
            .iter()
            .map(|e| binostr::stats::serialize(e, format).len())
            .sum();
        let batch_data = binostr::stats::serialize_batch(batch, format);
        let batch_total = batch_data.len();

        // Compress individual
        let individual_compressed: usize = batch
            .iter()
            .map(|e| {
                let data = binostr::stats::serialize(e, format);
                zstd::encode_all(data.as_slice(), 3).unwrap().len()
            })
            .sum();

        // Compress batch
        let batch_compressed = zstd::encode_all(batch_data.as_slice(), 3).unwrap().len();

        println!(
            "{:<20} {:>10} {:>10} {:>10} {:>10}",
            format.name(),
            individual_total,
            batch_total,
            individual_compressed,
            batch_compressed
        );
    }

    println!("\nNote: Batch compression is typically much more efficient than");
    println!("compressing individual events due to cross-event redundancy.");
}
