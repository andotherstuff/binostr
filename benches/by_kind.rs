//! Serialization and deserialization throughput by Nostr event kind.

use binostr::stats::{self, Format};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

mod common;

fn bench_kind(c: &mut Criterion, kind: u16, name: &str) {
    let events = common::load_by_kind(kind, 100);
    if events.is_empty() {
        return;
    }
    let mut group = c.benchmark_group(format!("kind_{kind}_{name}"));
    group.throughput(Throughput::Elements(events.len() as u64));

    for format in Format::enabled() {
        group.bench_function(format!("serialize/{}", format.short_name()), |b| {
            b.iter(|| {
                for event in &events {
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
    println!("\n=== Kind {kind} ({name}) - {} events ===", events.len());
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

fn bench_kind_0(c: &mut Criterion) {
    bench_kind(c, 0, "profile");
}
fn bench_kind_1(c: &mut Criterion) {
    bench_kind(c, 1, "notes");
}
fn bench_kind_3(c: &mut Criterion) {
    bench_kind(c, 3, "follows");
}
fn bench_kind_4(c: &mut Criterion) {
    bench_kind(c, 4, "dms");
}
fn bench_kind_7(c: &mut Criterion) {
    bench_kind(c, 7, "reactions");
}
fn bench_kind_10002(c: &mut Criterion) {
    bench_kind(c, 10002, "relays");
}
fn bench_kind_30023(c: &mut Criterion) {
    bench_kind(c, 30023, "articles");
}

criterion_group! {
    name = benches;
    config = common::auto_criterion();
    targets = bench_kind_0, bench_kind_1, bench_kind_3, bench_kind_4, bench_kind_7,
              bench_kind_10002, bench_kind_30023
}
criterion_main!(benches);
