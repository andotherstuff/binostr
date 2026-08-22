# Criterion repeatability across independent runs

This report compares 3 independent publication-profile Criterion runs on 1,000 events. Values are point estimates from each run; the per-run JSON files retain each Criterion confidence interval.

Campaign: `m4max-2026-08-21`. Commit: `0251b60ab9aea1d804a45b5f14ec62a46ff651b3` (dirty worktree). Toolchain: `rustc 1.96.1 (31fca3adb 2026-06-26) (Homebrew)`. Host: `Darwin 25.6.0 arm64; Apple M4 Max`.

Seed: `4776434982410539521`. Event-ID fingerprint: `a05ceb17de26aca15c5b284652a6e55188b0c5a135fb89839459d9d93ae492f5`. Run labels: `run-1`, `run-2`, `run-3`.

## Owned decode and validated ingest

| Format | Owned decode runs (µs/event) | Median | Range | Fully validated runs (µs/event) | Median | Range |
|---|---:|---:|---:|---:|---:|---:|
| json | 4.236, 4.196, 4.215 | 4.215 | 0.94% | 28.038, 28.055, 28.110 | 28.055 | 0.26% |
| cbor_packed | 5.844, 5.912, 5.863 | 5.863 | 1.16% | 29.827, 29.817, 29.779 | 29.817 | 0.16% |
| msgpack | 4.645, 4.682, 4.660 | 4.660 | 0.79% | 28.629, 28.548, 28.456 | 28.548 | 0.60% |
| flatbuffers | 6.137, 6.166, 6.127 | 6.137 | 0.64% | 30.073, 29.979, 30.187 | 30.073 | 0.69% |
| flexbuffers | 5.226, 5.229, 5.223 | 5.226 | 0.11% | 29.077, 29.032, 29.199 | 29.077 | 0.57% |
| avro | 6.671, 6.692, 6.661 | 6.671 | 0.47% | 30.504, 30.964, 30.550 | 30.550 | 1.51% |
| bson | 10.473, 10.415, 10.444 | 10.444 | 0.56% | 34.417, 34.508, 34.561 | 34.508 | 0.42% |
| thrift_compact | 4.937, 4.942, 4.936 | 4.937 | 0.14% | 28.697, 28.779, 28.748 | 28.748 | 0.28% |
| beve | 5.869, 5.858, 5.822 | 5.858 | 0.80% | 29.602, 29.676, 29.705 | 29.676 | 0.35% |
| bincode | 4.546, 4.528, 4.544 | 4.544 | 0.39% | 28.276, 28.294, 28.331 | 28.294 | 0.19% |
| postcard | 4.494, 4.492, 4.478 | 4.492 | 0.36% | 28.380, 28.190, 28.328 | 28.328 | 0.67% |
| proto_binary | 6.020, 6.042, 5.982 | 6.020 | 0.99% | 29.632, 29.614, 29.798 | 29.632 | 0.62% |
| capnp_packed | 3.777, 3.738, 3.740 | 3.740 | 1.04% | 27.429, 27.358, 27.588 | 27.429 | 0.84% |
| dannypack | 2.059, 2.057, 2.072 | 2.059 | 0.75% | 25.643, 25.661, 25.762 | 25.661 | 0.46% |
| notepack | 3.762, 3.754, 3.755 | 3.755 | 0.21% | 27.471, 27.531, 27.584 | 27.531 | 0.41% |

## Selective paths

| Path | Safety contract | Runs (ns/event) | Median | Range |
|---|---|---:|---:|---:|
| Notepack checked selective kind + pubkey | selected-field parser checks | 11.4, 11.5, 11.3 | 11.4 | 2.31% |
| FlatBuffers selective, exact buffer preverified | exact immutable buffer verified earlier | 17.0, 16.7, 16.7 | 16.7 | 1.72% |
| FlexBuffers checked selective kind + pubkey | root, vector, bounds, and selected types checked; skipped subtrees not fully verified | 18.2, 18.3, 18.2 | 18.2 | 0.56% |
| Cap’n Proto selective checked | message reader and selected fields checked | 137.2, 137.6, 137.2 | 137.2 | 0.28% |
| BEVE selective, exact buffer preverified | exact immutable buffer verified earlier; selected paths checked | 202.7, 200.5, 198.9 | 200.5 | 1.89% |
| BEVE selective, verify each read | whole value verified before selected paths | 891.3, 893.1, 884.2 | 891.3 | 1.00% |
| FlatBuffers selective, verify each read | whole buffer verified before selected fields | 2156.9, 2153.0, 2157.4 | 2156.9 | 0.20% |

A small cross-run range supports repeatability on this host, but it does not make close rankings portable across machines or runtimes. Compare safety contracts and full validation cost before interpreting raw codec ordering.
