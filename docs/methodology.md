# Benchmark methodology

## Question and unit of comparison

Binostr compares complete, semantically equivalent Nostr events. Every codec must preserve all seven NIP-01 fields and arbitrary tag strings. NIP-01 JSON is the compatibility baseline; binary encodings are transport or storage representations and do not replace the canonical JSON used to compute an event ID.

The primary decision metric is **fully validated events per second under bounded memory**. Raw encoding, owned decoding, wire size, allocations, tail latency, selective reads, compression, and batching are diagnostic metrics—not interchangeable definitions of “fastest.”

## Corpus and sampling

The tracked `data/sample.pb.gz` contains 50,000 length-delimited Protobuf events. All are unique by ID, and all 50,000 NIP-01 IDs and BIP-340 signatures verify; `results/corpus-audit.json` records the compressed-file SHA-256, complete kind distribution, and maximum content/tag dimensions. The repository history describes it as an eight-day sample of real Nostr traffic, but does not record the source relay, capture query, collection dates, license, consent process, or transformations before it was committed. Treat its provenance and representativeness as unknown until that metadata can be recovered. Do not use it to make claims about the entire Nostr network.

Publication reports:

1. load every `.pb.gz` file in lexicographic path order;
2. retain every event kind, including unknown and application-specific kinds;
3. select 1,000 events using a fixed `StdRng` seed; and
4. print and store the count, seed, and SHA-256 of the selected event IDs.

Unknown kinds used to be removed. That was inappropriate for a general wire-format comparison because their byte and tag distributions are still real workload inputs. Kind filtering remains available only for explicitly scoped experiments.

The default seed is `4776434982410539521` (`0x42494e4f53545201`). Set `BINOSTR_BENCH_SEED` to repeat the suite with another sample.

## Equivalent wire semantics

- Fixed ID, public-key, and signature fields are encoded as bytes when the format supports bytes.
- A lowercase, even-length hexadecimal tag value of at least eight characters may be represented as bytes. The wire type distinguishes it from text, and decode always reconstructs the exact lowercase string.
- Uppercase, mixed-case, odd-length, short, and non-hex strings remain text.
- All profiles preserve signed `created_at` values and the complete `u16` kind range used by the common model.
- Per-event compressed streams use the same four-byte big-endian length prefix before gzip or zstd. Concatenating unframed records would not be reconstructable and would favor formats with implicit boundaries.
- The input slice is one complete transport frame. Some base formats permit padding, unknown fields, concatenated values, or unreachable trailing bytes; the benchmark does not charge selected codecs for canonical re-encoding merely to prove byte-for-byte exhaustion. A production profile must specify whether such bytes are rejected at framing, accepted as a permitted base-format feature, or normalized before storage.
- Native batch encoding is measured separately from a length-framed sequence of individual records.
- Packed CBOR, MessagePack, FlexBuffers, and BEVE use the same typed positional encoder and reversible tag transform. This keeps their benchmark comparison focused on codec behavior instead of different event mappings. BEVE needs a format-specific typed-array tag visitor because its type-erased Serde decoder presents byte arrays as sequences.

Schemas and profiles live in `docs/`; the CBOR CDDL is paired with deterministic encoding and evolution rules in `docs/nostr.cbor.md`. Deterministic interoperability vectors live in `vectors/interop-v1.json`.

## Measurements

`examples/allocation_report.rs` measures allocator activity in an instrumented process. `examples/public_report.rs` then measures timings with the normal system allocator and joins the allocation artifact. Keeping these processes separate avoids adding atomic allocation counters to the timed paths. Together they record:

- owned encode and decode nanoseconds per event;
- decode plus NIP-01 ID recomputation;
- bounded decode, structure checks, ID recomputation, and BIP-340 verification;
- p50, p95, and p99 encode/decode latency, measured in 20-event chunks to reduce timer noise;
- allocation/reallocation counts and requested bytes;
- average record size and equally framed gzip/zstd stream size; and
- native batch size and encode/decode time.

The allocation figures are allocator requests, not retained heap or peak RSS. The latency percentiles describe chunks from one deterministic sample, not a production service under concurrency. The simple report runner warms each path and repeats full passes, while Criterion supplies statistical sampling, confidence intervals, regression comparisons, and outlier reporting for the dedicated benchmark suites.

## Correctness and robustness gates

- Registry-wide round trips cover semantic edge cases, batches, and 100 real events.
- The tracked corpus is checked for valid NIP-01 IDs and BIP-340 signatures.
- Every decoder is exercised under `catch_unwind` over every truncation of a valid record and deterministic hostile byte strings.
- `deserialize_limited` rejects oversized wire payloads before invoking a codec and checks content/tag limits after decode.
- Ten interoperable standard/envelope vectors are decoded by independent Python libraries; generated-schema formats use their normal Python code generation or runtime. Emerging and Rust/custom references are excluded from this interoperability claim.

This is robustness evidence, not a security proof or coverage-guided fuzzing campaign. Production SDKs should add continuous fuzzing, per-codec nesting/allocation limits, and concurrency/load testing.

## Reproduction

```bash
cargo test
cargo run --release --example audit_corpus
cargo run --release --example generate_vectors
python3 -m venv .venv-interop
.venv-interop/bin/pip install -r scripts/interop-requirements.txt
.venv-interop/bin/python scripts/verify_python_interop.py
cargo run --release --example allocation_report
cargo run --release --example public_report
BINOSTR_PUBLICATION_BENCH=1 cargo bench --bench zero_copy -- --noplot
BINOSTR_PUBLICATION_BENCH=1 cargo bench --bench validated -- --noplot
python3 scripts/export_criterion.py
python3 scripts/record_criterion_run.py record my-host-YYYYMMDD run-1
```

Repeat both Criterion commands and the export before recording `run-2` and `run-3` under the same new campaign label. The Python interoperability check additionally requires `protoc` and `flatc` on `PATH`. The multi-metric snapshot is written to `results/latest.{json,md}`. The Criterion exporter reads `target/criterion/*/*/new/estimates.json` and writes `results/validated-ingest.{json,md}` and `results/selective-access.{json,md}`. The run recorder preserves those confidence intervals and writes `results/criterion-repeatability.{json,md}`; the JSON artifacts are authoritative.

For a publication campaign, use an idle, powered machine; record power mode and thermal conditions externally; run Criterion at least three times; retain `target/criterion`; and do not compare absolute times across different hosts. The tracked campaign has three independent runs, but power and thermal state were not recorded, so it remains reproducible local evidence rather than a controlled hardware-lab result.

Set `BINOSTR_PUBLICATION_BENCH=1` for the repository's publication profile: 100 samples, a ten-second target measurement, a five-second warm-up, 95% confidence, and a 1% noise threshold. The environment variable takes precedence if the fast profile is also set.
