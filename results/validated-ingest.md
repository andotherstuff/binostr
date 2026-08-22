# Validated-ingest Criterion snapshot

Criterion run on 1,000 events: seed `4776434982410539521`, event-ID fingerprint `a05ceb17de26aca15c5b284652a6e55188b0c5a135fb89839459d9d93ae492f5`.

Criterion estimates completed `2026-08-21T23:15:01Z` and were exported `2026-08-21T23:15:07Z` from commit `0251b60ab9aea1d804a45b5f14ec62a46ff651b3` with a dirty worktree, using `rustc 1.96.1 (31fca3adb 2026-06-26) (Homebrew)` on `Darwin 25.6.0 arm64; Apple M4 Max`. This is one publication-profile run; power and thermal state were not recorded.

Command: `BINOSTR_PUBLICATION_BENCH=1 cargo bench --bench validated -- --noplot`. The publication profile uses 100 samples, a five-second warm-up, a ten-second target measurement, and 95% confidence intervals. Each interval is the time for 1,000 events; divide milliseconds by 1,000 to obtain microseconds/event.

| Format | Bounded decode + ID | Bounded decode + ID + signature |
|---|---:|---:|
| JSON | 9.8101–9.8366 ms | 28.0692–28.1520 ms |
| CBOR packed | 11.4755–11.5002 ms | 29.7359–29.8277 ms |
| MessagePack | 10.2723–10.2994 ms | 28.4177–28.4939 ms |
| FlatBuffers | 11.7820–11.8488 ms | 30.0984–30.2824 ms |
| FlexBuffers | 10.8696–10.8922 ms | 29.0910–29.3591 ms |
| Avro binary datum | 12.2499–12.2799 ms | 30.4983–30.6069 ms |
| BSON | 16.1157–16.1526 ms | 34.5045–34.6249 ms |
| Thrift Compact | 10.5590–10.6068 ms | 28.7159–28.7808 ms |
| BEVE emerging reference | 11.4731–11.4983 ms | 29.6755–29.7338 ms |
| bincode reference | 10.1334–10.1628 ms | 28.2806–28.3852 ms |
| postcard reference | 10.1225–10.1511 ms | 28.2904–28.3672 ms |
| Protocol Buffers binary | 11.6193–11.6466 ms | 29.7603–29.8364 ms |
| Cap’n Proto packed custom profile | 9.4959–9.5879 ms | 27.5497–27.6309 ms |
| DannyPack custom reference | 7.6111–7.6297 ms | 25.7212–25.8066 ms |
| Notepack custom reference | 9.3793–9.4094 ms | 27.5441–27.6294 ms |

Criterion’s raw sample and outlier evidence remains under `target/criterion`; retain it when making a close comparison. Codec differences remain visible after decode plus ID, but BIP-340 verification substantially narrows their share of accepted-event cost.
