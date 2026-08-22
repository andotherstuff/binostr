//! Throughput by event-size and tag-count category.

use binostr::event::{SizeCategory, TagCategory};
use binostr::stats::{self, Format};
use binostr::{EventSampler, NostrEvent};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

mod common;

const SAMPLE_SIZE: usize = 100;

fn sampler() -> Option<EventSampler> {
    match EventSampler::from_directory_seeded(common::DATA_DIR, common::benchmark_seed()) {
        Ok(value) => Some(value),
        Err(error) => {
            eprintln!("Could not load events: {error}");
            None
        }
    }
}

fn load_by_size(category: SizeCategory) -> Vec<NostrEvent> {
    let Some(mut sampler) = sampler() else {
        return common::generate_synthetic_events(SAMPLE_SIZE);
    };
    sampler
        .sample_size(category, SAMPLE_SIZE)
        .into_iter()
        .cloned()
        .collect()
}

fn load_by_tags(category: TagCategory) -> Vec<NostrEvent> {
    let Some(mut sampler) = sampler() else {
        return common::generate_synthetic_events(SAMPLE_SIZE);
    };
    sampler
        .sample_tags(category, SAMPLE_SIZE)
        .into_iter()
        .cloned()
        .collect()
}

fn bench_events(c: &mut Criterion, group_name: &str, events: &[NostrEvent]) {
    if events.is_empty() {
        return;
    }
    let mut group = c.benchmark_group(group_name);
    group.throughput(Throughput::Elements(events.len() as u64));
    for format in Format::enabled() {
        group.bench_function(format!("serialize/{}", format.short_name()), |b| {
            b.iter(|| {
                for event in events {
                    black_box(stats::serialize(event, format));
                }
            })
        });
        let encoded: Vec<_> = events
            .iter()
            .map(|event| stats::serialize(event, format))
            .collect();
        group.bench_function(format!("deserialize/{}", format.short_name()), |b| {
            b.iter(|| {
                for data in &encoded {
                    black_box(stats::deserialize(data, format).unwrap());
                }
            })
        });
    }
    group.finish();

    let json_total: usize = events
        .iter()
        .map(|e| stats::serialize(e, Format::Json).len())
        .sum();
    println!("\n=== {group_name} - {} events ===", events.len());
    for format in Format::enabled() {
        let total: usize = events
            .iter()
            .map(|e| stats::serialize(e, format).len())
            .sum();
        println!(
            "  {:20} {:>7.1} bytes ({:>5.1}%)",
            format.name(),
            total as f64 / events.len() as f64,
            100.0 * total as f64 / json_total as f64
        );
    }
}

fn bench_size(c: &mut Criterion, category: SizeCategory, name: &str) {
    bench_events(c, &format!("size_{name}"), &load_by_size(category));
}
fn bench_tags(c: &mut Criterion, category: TagCategory, name: &str) {
    bench_events(c, &format!("tags_{name}"), &load_by_tags(category));
}

fn size_tiny(c: &mut Criterion) {
    bench_size(c, SizeCategory::Tiny, "tiny");
}
fn size_small(c: &mut Criterion) {
    bench_size(c, SizeCategory::Small, "small");
}
fn size_medium(c: &mut Criterion) {
    bench_size(c, SizeCategory::Medium, "medium");
}
fn size_large(c: &mut Criterion) {
    bench_size(c, SizeCategory::Large, "large");
}
fn size_huge(c: &mut Criterion) {
    bench_size(c, SizeCategory::Huge, "huge");
}
fn tags_none(c: &mut Criterion) {
    bench_tags(c, TagCategory::None, "none");
}
fn tags_few(c: &mut Criterion) {
    bench_tags(c, TagCategory::Few, "few");
}
fn tags_moderate(c: &mut Criterion) {
    bench_tags(c, TagCategory::Moderate, "moderate");
}
fn tags_many(c: &mut Criterion) {
    bench_tags(c, TagCategory::Many, "many");
}
fn tags_massive(c: &mut Criterion) {
    bench_tags(c, TagCategory::Massive, "massive");
}

criterion_group! { name = size_benches; config = common::auto_criterion(); targets = size_tiny, size_small, size_medium, size_large, size_huge }
criterion_group! { name = tag_benches; config = common::auto_criterion(); targets = tags_none, tags_few, tags_moderate, tags_many, tags_massive }
criterion_main!(size_benches, tag_benches);
