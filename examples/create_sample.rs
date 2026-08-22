//! Create a smaller sample file from all data files
//!
//! Usage: cargo run --release --example create_sample -- [num_events]
//! Default: 50000 events

use binostr::proto_gen::nostr::ProtoEvent;
use binostr::{EventLoader, EXCLUDED_KINDS};
use flate2::write::GzEncoder;
use flate2::Compression;
use prost::Message;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::fs::{self, File};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let num_events: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(50000);

    println!("Creating sample file with {} events...", num_events);

    // Load ALL events from all files
    let mut all_events = Vec::new();

    for entry in fs::read_dir("data")? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        if !path.is_file() || !name.ends_with(".pb.gz") || name.starts_with("sample") {
            continue;
        }

        println!("Loading from {}...", path.display());

        let mut loader = EventLoader::open(&path)?;
        let mut count = 0;
        let mut errors = 0;

        loop {
            match loader.next_event() {
                Ok(Some(event)) => {
                    all_events.push(event);
                    count += 1;
                }
                Ok(None) => break, // EOF
                Err(_) => {
                    errors += 1;
                    if errors > 1000 {
                        println!("  Too many errors, moving to next file");
                        break;
                    }
                }
            }
        }

        println!("  Loaded {} events (skipped {} errors)", count, errors);
    }

    println!("\nTotal events loaded: {}", all_events.len());

    // Filter out excluded/unknown kinds
    let before_filter = all_events.len();
    all_events.retain(|e| !EXCLUDED_KINDS.contains(&e.kind));
    println!(
        "Filtered out {} unknown/excluded kinds",
        before_filter - all_events.len()
    );
    println!("Events after filtering: {}", all_events.len());

    if all_events.len() < num_events {
        println!(
            "Warning: Only {} events available, using all of them",
            all_events.len()
        );
    }

    // Shuffle and take sample
    let mut rng = rand::rngs::StdRng::seed_from_u64(42); // Deterministic for reproducibility
    all_events.shuffle(&mut rng);
    let sample: Vec<_> = all_events.into_iter().take(num_events).collect();

    println!("Selected {} random events", sample.len());

    // Write to new file using varint length prefix (same format as input)
    let output_path = "data/sample.pb.gz";
    let file = File::create(output_path)?;
    let mut encoder = GzEncoder::new(file, Compression::default());

    for event in &sample {
        let proto_event = ProtoEvent {
            id: hex::encode(event.id),
            pubkey: hex::encode(event.pubkey),
            created_at: event.created_at,
            kind: event.kind as i32,
            tags: event
                .tags
                .iter()
                .map(|t| binostr::proto_gen::nostr::Tag { values: t.clone() })
                .collect(),
            content: event.content.clone(),
            sig: hex::encode(event.sig),
        };

        let buf = proto_event.encode_to_vec();
        // Write varint length prefix
        write_varint(&mut encoder, buf.len() as u64)?;
        encoder.write_all(&buf)?;
    }

    encoder.finish()?;

    // Print file size
    let metadata = std::fs::metadata(output_path)?;
    let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
    println!(
        "\nCreated {} with {} events ({:.1} MB)",
        output_path,
        sample.len(),
        size_mb
    );

    // Print kind distribution
    let mut kinds: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
    for event in &sample {
        *kinds.entry(event.kind).or_insert(0) += 1;
    }
    let mut kinds: Vec<_> = kinds.into_iter().collect();
    kinds.sort_by_key(|item| std::cmp::Reverse(item.1));

    println!("\nKind distribution:");
    for (kind, count) in kinds.iter().take(15) {
        println!(
            "  Kind {:>5}: {:>6} events ({:.1}%)",
            kind,
            count,
            100.0 * *count as f64 / sample.len() as f64
        );
    }

    println!("\n✓ You can now delete the other data files:");
    println!("  rm data/2025_09_*.pb.gz");

    Ok(())
}

/// Write a varint to a writer
fn write_varint<W: Write>(writer: &mut W, mut value: u64) -> std::io::Result<()> {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        writer.write_all(&[byte])?;
        if value == 0 {
            break;
        }
    }
    Ok(())
}
