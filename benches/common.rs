//! Common utilities for benchmarks

use binostr::stats::Format;
use binostr::{config, EventSampler, NostrEvent};
use criterion::Criterion;
use sha2::{Digest, Sha256};
use std::time::Duration;

/// Default Criterion configuration for benchmarks.
///
/// Uses Criterion's defaults which provide statistically sound results:
/// - 100 samples
/// - 5s measurement time
/// - 3s warm-up time
/// - 95% confidence level
#[allow(dead_code)]
pub fn default_criterion() -> Criterion {
    Criterion::default()
}

/// Create a fast Criterion configuration for quicker iterations during development.
///
/// Use by temporarily changing the benchmark config, or set env var:
/// `BINOSTR_FAST_BENCH=1 cargo bench`
///
/// Settings:
/// - 30 samples (default: 100) - faster iteration
/// - 2s measurement time (default: 5s) - quicker feedback
/// - 500ms warm-up (default: 3s) - reduced warm-up
/// - 90% confidence (default: 95%) - acceptable for development
#[allow(dead_code)]
pub fn fast_criterion() -> Criterion {
    Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(2))
        .warm_up_time(Duration::from_millis(500))
        .confidence_level(0.90)
}

/// Create a publication-quality Criterion configuration for final benchmark runs.
///
/// More rigorous than defaults for publishable results:
/// - 100 samples - statistically robust
/// - 10s measurement time - thorough measurement
/// - 5s warm-up - ensure CPU caches are hot
/// - 95% confidence - publication-quality significance
/// - Noise threshold 0.01 - detect small improvements
#[allow(dead_code)]
pub fn publication_criterion() -> Criterion {
    Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(5))
        .confidence_level(0.95)
        .noise_threshold(0.01)
        .significance_level(0.05)
}

/// Select Criterion config based on environment.
///
/// `BINOSTR_PUBLICATION_BENCH=1` takes precedence over
/// `BINOSTR_FAST_BENCH=1` if both are present.
pub fn auto_criterion() -> Criterion {
    if std::env::var("BINOSTR_PUBLICATION_BENCH").is_ok() {
        publication_criterion()
    } else if std::env::var("BINOSTR_FAST_BENCH").is_ok() {
        fast_criterion()
    } else {
        default_criterion()
    }
}

/// Check if a format is enabled in config (by short name like "json", "cbor_packed", etc.)
#[allow(dead_code)]
pub fn is_enabled(format_name: &str) -> bool {
    config::is_format_enabled(format_name)
}

/// Get all enabled formats
#[allow(dead_code)]
pub fn enabled_formats() -> Vec<Format> {
    Format::enabled()
}

/// Print which formats are enabled at benchmark start
#[allow(dead_code)]
pub fn print_enabled_formats() {
    let enabled: Vec<_> = Format::enabled().iter().map(|f| f.name()).collect();
    eprintln!("Enabled formats: {}", enabled.join(", "));
}

/// Macro to conditionally run a benchmark only if the format is enabled
///
/// Usage:
/// ```ignore
/// bench_if_enabled!(group, "json", |b| {
///     b.iter(|| json::serialize(black_box(event)))
/// });
/// ```
#[macro_export]
macro_rules! bench_if_enabled {
    ($group:expr, $format:expr, $bench:expr) => {
        if common::is_enabled($format) {
            $group.bench_function($format, $bench);
        }
    };
    ($group:expr, $format:expr, $id:expr, $input:expr, $bench:expr) => {
        if common::is_enabled($format) {
            $group.bench_with_input($id, $input, $bench);
        }
    };
}

/// Default data directory
pub const DATA_DIR: &str = "data";

/// Stable default seed for publication-quality, reproducible samples.
pub const DEFAULT_BENCH_SEED: u64 = 0x4249_4e4f_5354_5201;

pub fn benchmark_seed() -> u64 {
    std::env::var("BINOSTR_BENCH_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_BENCH_SEED)
}

fn load_sampler() -> Result<EventSampler, binostr::loader::LoadError> {
    EventSampler::from_directory_seeded(DATA_DIR, benchmark_seed())
}

fn corpus_fingerprint(events: &[NostrEvent]) -> String {
    let mut hasher = Sha256::new();
    for event in events {
        hasher.update(event.id);
    }
    hex::encode(hasher.finalize())
}

fn report_sample(events: &[NostrEvent]) {
    eprintln!(
        "Benchmark sample: events={}, seed={}, id_sha256={}",
        events.len(),
        benchmark_seed(),
        corpus_fingerprint(events)
    );
}

/// Load a sample of events for benchmarking
#[allow(dead_code)]
pub fn load_sample(size: usize) -> Vec<NostrEvent> {
    let mut sampler = match load_sampler() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: Could not load events from {}: {}", DATA_DIR, e);
            eprintln!("Using synthetic test events instead.");
            return generate_synthetic_events(size);
        }
    };

    if sampler.len() < size {
        eprintln!(
            "Warning: Only loaded {} events, requested {}",
            sampler.len(),
            size
        );
    }

    let events: Vec<_> = sampler.random_sample(size).into_iter().cloned().collect();
    report_sample(&events);
    events
}

/// Load events filtered by kind
#[allow(dead_code)]
pub fn load_by_kind(kind: u16, size: usize) -> Vec<NostrEvent> {
    let mut sampler = match load_sampler() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: Could not load events: {}", e);
            return generate_synthetic_events_kind(kind, size);
        }
    };

    let events = sampler.sample_kind(kind, size);
    if events.is_empty() {
        eprintln!("Warning: No events found for kind {}", kind);
        return generate_synthetic_events_kind(kind, size);
    }

    let events: Vec<_> = events.into_iter().cloned().collect();
    report_sample(&events);
    events
}

/// Generate synthetic events for testing when data files aren't available
#[allow(dead_code)]
pub fn generate_synthetic_events(count: usize) -> Vec<NostrEvent> {
    (0..count)
        .map(|i| NostrEvent {
            id: {
                let mut arr = [0u8; 32];
                arr[0..8].copy_from_slice(&(i as u64).to_le_bytes());
                arr
            },
            pubkey: [0xab; 32],
            created_at: 1700000000 + i as i64,
            kind: 1,
            tags: vec![
                vec!["p".to_string(), hex::encode([0xcd; 32])],
                vec!["e".to_string(), hex::encode([0xef; 32])],
            ],
            content: format!("This is test event number {}. Hello Nostr!", i),
            sig: [0x12; 64],
        })
        .collect()
}

/// Generate synthetic events of a specific kind
#[allow(dead_code)]
pub fn generate_synthetic_events_kind(kind: u16, count: usize) -> Vec<NostrEvent> {
    (0..count)
        .map(|i| {
            let (tags, content) = match kind {
                0 => {
                    // Profile metadata
                    (
                        vec![],
                        format!(
                            r#"{{"name":"user{}","about":"A test user","picture":"https://example.com/pic.jpg"}}"#,
                            i
                        ),
                    )
                }
                1 => {
                    // Short text note
                    (
                        vec![
                            vec!["p".to_string(), hex::encode([i as u8; 32])],
                            vec!["t".to_string(), "nostr".to_string()],
                        ],
                        format!("This is test note {}. #nostr", i),
                    )
                }
                3 => {
                    // Follow list (many p tags)
                    let tags: Vec<Vec<String>> = (0..100)
                        .map(|j| {
                            vec![
                                "p".to_string(),
                                hex::encode([(i + j) as u8; 32]),
                                "wss://relay.example.com".to_string(),
                            ]
                        })
                        .collect();
                    (tags, String::new())
                }
                7 => {
                    // Reaction
                    (
                        vec![
                            vec!["e".to_string(), hex::encode([i as u8; 32])],
                            vec!["p".to_string(), hex::encode([(i + 1) as u8; 32])],
                        ],
                        "🤙".to_string(),
                    )
                }
                30023 => {
                    // Long-form article
                    let content = format!(
                        "# Article {}\n\n{}\n\n## Section 1\n\n{}\n\n## Section 2\n\n{}",
                        i,
                        "Lorem ipsum ".repeat(50),
                        "Dolor sit amet ".repeat(100),
                        "Consectetur adipiscing ".repeat(100),
                    );
                    (
                        vec![
                            vec!["d".to_string(), format!("article-{}", i)],
                            vec!["title".to_string(), format!("Test Article {}", i)],
                            vec!["published_at".to_string(), "1700000000".to_string()],
                        ],
                        content,
                    )
                }
                _ => {
                    // Generic event
                    (vec![], format!("Kind {} event {}", kind, i))
                }
            };

            NostrEvent {
                id: {
                    let mut arr = [0u8; 32];
                    arr[0..8].copy_from_slice(&(i as u64).to_le_bytes());
                    arr
                },
                pubkey: [0xab; 32],
                created_at: 1700000000 + i as i64,
                kind,
                tags,
                content,
                sig: [0x12; 64],
            }
        })
        .collect()
}
