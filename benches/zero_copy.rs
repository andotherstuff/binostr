//! Selective relay-filter field access, separated from full materialization.

use binostr::stats::{self, Format};
use binostr::{beve, capnp, flatbuffers, flexbuffers, notepack};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

mod common;

fn bench_selective_access(c: &mut Criterion) {
    let events = common::load_sample(1000);
    if events.is_empty() {
        return;
    }
    let mut group = c.benchmark_group("read_kind_and_pubkey");
    group.throughput(Throughput::Elements(events.len() as u64));

    for format in Format::enabled() {
        let encoded: Vec<_> = events
            .iter()
            .map(|event| stats::serialize(event, format))
            .collect();
        group.bench_function(format!("{}/full_materialize", format.short_name()), |b| {
            b.iter(|| {
                for data in &encoded {
                    let event = stats::deserialize(data, format).unwrap();
                    black_box((event.kind, event.pubkey));
                }
            })
        });
    }

    let capnp_data: Vec<_> = events.iter().map(capnp::serialize_event).collect();
    group.bench_function("capnp/selective_checked", |b| {
        b.iter(|| {
            for data in &capnp_data {
                black_box(capnp::read_kind_and_pubkey(data).unwrap());
            }
        })
    });

    let flat_data: Vec<_> = events.iter().map(flatbuffers::serialize).collect();
    group.bench_function("flatbuffers/selective_verify_each", |b| {
        b.iter(|| {
            for data in &flat_data {
                black_box(flatbuffers::read_kind_and_pubkey(data).unwrap());
            }
        })
    });

    let flex_data: Vec<_> = events.iter().map(flexbuffers::serialize).collect();
    group.bench_function("flexbuffers/selective_checked", |b| {
        b.iter(|| {
            for data in &flex_data {
                black_box(flexbuffers::read_kind_and_pubkey(data).unwrap());
            }
        })
    });

    let beve_data: Vec<_> = events.iter().map(beve::serialize).collect();
    group.bench_function("beve/selective_verify_each", |b| {
        b.iter(|| {
            for data in &beve_data {
                beve::verify(data).unwrap();
                black_box(beve::read_kind_and_pubkey(data).unwrap());
            }
        })
    });
    for data in &beve_data {
        beve::verify(data).unwrap();
    }
    group.bench_function("beve/selective_preverified", |b| {
        b.iter(|| {
            for data in &beve_data {
                black_box(beve::read_kind_and_pubkey(data).unwrap());
            }
        })
    });
    for data in &flat_data {
        flatbuffers::verify(data).unwrap();
    }
    group.bench_function("flatbuffers/selective_preverified", |b| {
        b.iter(|| {
            for data in &flat_data {
                // SAFETY: every immutable buffer was verified immediately above.
                black_box(unsafe { flatbuffers::read_kind_and_pubkey_trusted(data) });
            }
        })
    });

    let notepack_data: Vec<_> = events.iter().map(notepack::serialize).collect();
    group.bench_function("notepack/selective_checked", |b| {
        b.iter(|| {
            for data in &notepack_data {
                black_box(notepack::read_kind_and_pubkey(data).unwrap());
            }
        })
    });
    group.finish();
}

criterion_group! { name = benches; config = common::auto_criterion(); targets = bench_selective_access }
criterion_main!(benches);
