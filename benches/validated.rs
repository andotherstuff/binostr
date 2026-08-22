//! Criterion measurements for work that produces a policy-usable event.

use binostr::stats::{self, Format};
use binostr::validation::{self, EventLimits};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

mod common;

fn bench_validated(c: &mut Criterion) {
    let events = common::load_sample(1000);
    if events.is_empty() {
        return;
    }
    let limits = EventLimits::default();
    let mut group = c.benchmark_group("validated_ingest");
    group.throughput(Throughput::Elements(events.len() as u64));

    for format in Format::enabled() {
        let encoded: Vec<_> = events
            .iter()
            .map(|event| stats::serialize(event, format))
            .collect();
        group.bench_function(format!("{}/decode_id", format.short_name()), |b| {
            b.iter(|| {
                for data in &encoded {
                    let event = stats::deserialize_limited(data, format, &limits).unwrap();
                    validation::verify_id(&event).unwrap();
                    black_box(event);
                }
            })
        });
        group.bench_function(
            format!("{}/decode_id_signature", format.short_name()),
            |b| {
                b.iter(|| {
                    for data in &encoded {
                        let event = stats::deserialize_limited(data, format, &limits).unwrap();
                        validation::verify_id_and_signature(&event).unwrap();
                        black_box(event);
                    }
                })
            },
        );
    }
    group.finish();
}

criterion_group! { name = benches; config = common::auto_criterion(); targets = bench_validated }
criterion_main!(benches);
