# Binostr

Binostr is a reproducible benchmark, robustness suite, and interoperability test bed for binary representations of [Nostr NIP-01 events](https://github.com/nostr-protocol/nips/blob/master/01.md).

The project is standards-first. A custom wire format is a useful performance reference, but not the default choice unless its end-to-end benefit survives validation, framing, allocation, interoperability, evolution, and maintenance costs.

## What the current data says

The latest release-mode snapshot uses 1,000 seeded real events and compares 15 enabled profiles. Complete data, host metadata, allocations, tail latency, batching, and compression are in [results/latest.json](results/latest.json); the readable table is [results/latest.md](results/latest.md). Three independent publication-profile Criterion runs and their cross-run ranges are in [results/criterion-repeatability.md](results/criterion-repeatability.md).

- Packed CBOR and MessagePack are effectively identical in raw size: about 1,686 bytes/event, or 64.4% of JSON on this sample.
- Across three independent publication-profile runs, median owned decode was 4.660 µs/event for MessagePack, 4.937 for Thrift Compact, 5.226 for FlexBuffers, 5.863 for packed CBOR, 6.020 for Protobuf, 6.137 for FlatBuffers, 6.671 for Avro, and 10.444 for BSON. The largest owned-decode range among all 15 profiles was 1.16%. CBOR, MessagePack, and FlexBuffers share one typed positional model, so their comparison does not confound the codecs with different Rust representations.
- Full validation changes the decision picture. Three-run medians were 28.548 µs/event for MessagePack, 28.748 for Thrift Compact, 29.077 for FlexBuffers, and 29.817 for packed CBOR after bounded decode, structural checks, NIP-01 ID recomputation, and BIP-340 verification; BEVE’s emerging reference was 29.676 and the custom DannyPack reference 25.661 µs. The largest full-validation range among all profiles was 1.51%. Cryptography and canonicalization materially narrow the raw-decode spread. See [results/criterion-repeatability.md](results/criterion-repeatability.md); the latest per-run confidence intervals are in [results/validated-ingest.md](results/validated-ingest.md).
- Equally length-framed zstd streams for packed CBOR, MessagePack, Avro, Thrift Compact, BEVE, bincode, postcard, Protobuf, DannyPack, and Notepack are within about 1% of one another. Raw wire size does not predict compressed-stream cost particularly well.
- Borrowed selective reads are a different workload. Three-run medians on the same corpus were 11.4 ns/event for checked Notepack kind+pubkey reads, 16.7 for preverified FlatBuffers, 18.2 for checked FlexBuffers, and 137.2 for checked Cap’n Proto, versus microseconds for full object materialization. These paths have different safety contracts; see [results/criterion-repeatability.md](results/criterion-repeatability.md) for cross-run ranges and [results/selective-access.md](results/selective-access.md) for the latest confidence intervals.
- Ten Rust-generated interoperable standard/envelope vectors are decoded successfully by independent Python libraries. This covers JSON, CBOR, MessagePack, FlexBuffers, Protobuf, FlatBuffers, Avro, BSON, Thrift Compact, and Cap’n Proto packed. BEVE is measured only as an emerging reference and is excluded from this claim.

These are measurements of the implementations and profiles in this repository, not intrinsic format ceilings. The current tracked results were produced from a dirty development tree and say so in their metadata; regenerate them from the final clean revision before citing them as a release artifact.

## Recommendation

Start protocol design with the deterministic packed **CBOR** profile because it combines IETF governance, broad implementation availability, competitive size, and a simple standard data model. Keep **MessagePack** as the closest alternative: it has very broad practical support and this Rust implementation currently decodes about 21% faster at essentially the same size. Both now use the same typed positional model and allocation count; CBOR allocates modestly more bytes while decoding this corpus.

Choose between them using the actual SDK language matrix—especially Swift, Kotlin, JavaScript/TypeScript, Rust, and the server languages—not the Rust row alone. Protocol Buffers remains attractive when generated schemas and explicit evolution discipline are desired. FlatBuffers and Cap’n Proto deserve special consideration only if the SDK exposes verified borrowed views and the real workload repeatedly filters already-validated buffers.

The SDK should optimize **validated useful events per second under bounded memory**, not raw deserialization in isolation. Keep framing, codec, semantic validation, policy, and ownership as separate layers so a wire-format choice does not infect the entire event model. See [docs/sdk-design.md](docs/sdk-design.md).

## Measured formats

| Category | Profiles |
|---|---|
| Nostr baseline | NIP-01 JSON |
| Open interoperability candidates | CBOR packed, MessagePack, Protocol Buffers binary, FlatBuffers, FlexBuffers, Apache Avro binary datum, BSON, Apache Thrift Compact |
| Emerging open-format reference | BEVE Version 2 |
| Standard envelope with custom Nostr packing | Cap’n Proto packed |
| Rust-only references | bincode, postcard |
| Nostr-specific references | DannyPack, Notepack |
| Disabled legacy/profile variants | CBOR string-keyed and integer-keyed maps, Protobuf string fields, unpacked Cap’n Proto |

Why these were included, what else was considered, and the current selection policy are documented in [docs/format-selection.md](docs/format-selection.md). Wire definitions live beside it as CDDL, `.proto`, `.fbs`, `.avsc`, `.thrift`, and `.capnp` files, with explicit profiles for [CBOR](docs/nostr.cbor.md), [MessagePack](docs/nostr.msgpack.md), [FlexBuffers](docs/nostr.flexbuffers.md), and the [BEVE reference](docs/nostr.beve.md).

## Reproduce

Prerequisites are a current stable Rust toolchain and the Cap’n Proto and Protobuf compilers used by `build.rs`.

```bash
# Correctness, real-corpus round trips, validation, and hostile-input tests
cargo test

# Allocation instrumentation runs separately so it cannot skew timings
cargo run --release --example allocation_report

# Publication snapshot (reads allocations.json; writes latest.{json,md})
cargo run --release --example public_report

# Statistically sampled benchmark suites
cargo bench --bench serialize
cargo bench --bench deserialize
cargo bench --bench by_kind
cargo bench --bench by_category
cargo bench --bench zero_copy
cargo bench --bench validated
cargo bench --bench size_analysis

# Publication-profile confidence intervals and tracked summaries
BINOSTR_PUBLICATION_BENCH=1 cargo bench --bench zero_copy -- --noplot
BINOSTR_PUBLICATION_BENCH=1 cargo bench --bench validated -- --noplot
python3 scripts/export_criterion.py

# Record each of three independent runs under one new campaign label
python3 scripts/record_criterion_run.py record my-host-YYYYMMDD run-1
# Repeat both publication benches and export before recording run-2 and run-3.
```

The benchmark seed is stable. Override it for another reproducible sample:

```bash
BINOSTR_BENCH_SEED=12345 cargo run --release --example public_report
BINOSTR_BENCH_SEED=12345 cargo bench
```

Every run prints or stores the event count, seed, and SHA-256 fingerprint of selected event IDs. Preserve all three with results. Set `BINOSTR_PUBLICATION_BENCH=1` for 100 samples, a ten-second target measurement, five-second warm-up, and 95% confidence intervals. `BINOSTR_FAST_BENCH=1` shortens Criterion runs for development and reduces their statistical rigor.

### Independent Python interoperability

This check also requires `python3`, `protoc`, and `flatc`:

```bash
cargo run --release --example generate_vectors
python3 -m venv .venv-interop
.venv-interop/bin/pip install -r scripts/interop-requirements.txt
.venv-interop/bin/python scripts/verify_python_interop.py
```

The deterministic fixture is [vectors/interop-v1.json](vectors/interop-v1.json). Fixed dependency versions make the test repeatable; update them deliberately and regenerate the evidence.

## Methodology and limitations

[docs/methodology.md](docs/methodology.md) is the normative methodology. In short:

- every codec goes through one registry and must preserve the same semantic event;
- record-stream compression uses identical four-byte length framing;
- native batches are measured separately;
- reports distinguish owned decode, decode+ID, and full cryptographic validation;
- allocation requests and latency percentiles are first-class outputs; and
- decoder truncation and hostile-input tests run across every registered format.

The tracked corpus contains 50,000 unique events; all 50,000 IDs and BIP-340 signatures verify. Its content hash, complete kind distribution, and extrema are in [results/corpus-audit.json](results/corpus-audit.json). Repository history describes it as an eight-day real-traffic sample, but its relay/source, collection query and dates, redistribution terms, and transformations were never recorded. Its provenance and network representativeness are therefore unknown. Recover that metadata or replace it with a clearly licensed, reproducible corpus before treating this repository as a formal public study. See [data/README.md](data/README.md) for the data-governance record and replacement requirements.

## Repository map

```text
src/                    semantic model, codecs, registry, validation
docs/                   methodology, selection analysis, SDK design, schemas
benches/                Criterion throughput, category, and selective-read suites
examples/               report, vector, corpus, size, and batch tools
tests/                  registry-wide round trips and malformed-input checks
vectors/                deterministic cross-language wire fixtures
results/                snapshots, per-run confidence intervals, repeatability evidence
scripts/                interoperability and Criterion evidence exporters
data/                   tracked benchmark corpus
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). At minimum, run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, and the Python interoperability check.

## License

MIT. See [LICENSE](LICENSE). The code license does not establish provenance or redistribution rights for the tracked corpus; that is the outstanding data-governance issue described above.
