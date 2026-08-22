//! Convert sample.pb.gz to JSONL format
//!
//! Usage: cargo run --release --example pb_to_jsonl [input] [output]
//! Default input: data/sample.pb.gz
//! Default output: data/sample.jsonl

use binostr::{json, EventLoader};
use std::fs::File;
use std::io::{BufWriter, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let input = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("data/sample.pb.gz");
    let output = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("data/sample.jsonl");

    println!("Converting {} to {}", input, output);

    let loader = EventLoader::open(input)?;
    let out_file = File::create(output)?;
    let mut writer = BufWriter::new(out_file);

    let mut count = 0;
    let mut errors = 0;

    for result in loader {
        match result {
            Ok(event) => {
                let json_bytes = json::serialize(&event);
                writer.write_all(&json_bytes)?;
                writer.write_all(b"\n")?;
                count += 1;

                if count % 10000 == 0 {
                    print!("\rProcessed {} events...", count);
                    std::io::stdout().flush()?;
                }
            }
            Err(e) => {
                errors += 1;
                if errors <= 5 {
                    eprintln!("Error: {}", e);
                }
            }
        }
    }

    writer.flush()?;

    println!("\r                              ");
    println!("Wrote {} events to {}", count, output);
    if errors > 0 {
        println!("Skipped {} errors", errors);
    }

    // Print file sizes
    let input_size = std::fs::metadata(input)?.len();
    let output_size = std::fs::metadata(output)?.len();

    println!("\nFile sizes:");
    println!(
        "  Input:  {:.2} MB (compressed protobuf)",
        input_size as f64 / 1_000_000.0
    );
    println!(
        "  Output: {:.2} MB (JSONL)",
        output_size as f64 / 1_000_000.0
    );

    Ok(())
}
