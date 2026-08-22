# Selective-access snapshot

Criterion run on the same 1,000-event corpus as `latest.json`: seed `4776434982410539521`, event-ID fingerprint `a05ceb17de26aca15c5b284652a6e55188b0c5a135fb89839459d9d93ae492f5`.

Criterion estimates completed `2026-08-21T23:05:55Z` and were exported `2026-08-21T23:15:07Z` from commit `0251b60ab9aea1d804a45b5f14ec62a46ff651b3` with a dirty worktree, using `rustc 1.96.1 (31fca3adb 2026-06-26) (Homebrew)` on `Darwin 25.6.0 arm64; Apple M4 Max`. This is one publication-profile run; power and thermal state were not recorded.

Command: `BINOSTR_PUBLICATION_BENCH=1 cargo bench --bench zero_copy -- --noplot`. The publication profile uses 100 samples, a five-second warm-up, a ten-second target measurement, and 95% confidence intervals. Times below are for 1,000 events; the last column normalizes the midpoint per event.

| Path | Safety/materialization contract | Criterion interval | Approx. ns/event |
|---|---|---:|---:|
| Notepack checked selective kind + pubkey | selected-field parser checks | 11.24–11.29 µs | 11.3 |
| FlatBuffers selective, exact buffer preverified | exact immutable buffer verified earlier | 16.70–16.75 µs | 16.7 |
| FlexBuffers checked selective kind + pubkey | root, vector, bounds, and selected types checked; skipped subtrees not fully verified | 18.11–18.22 µs | 18.2 |
| Cap’n Proto selective checked | message reader and selected fields checked | 136.71–137.95 µs | 137.2 |
| BEVE selective, exact buffer preverified | exact immutable buffer verified earlier; selected paths checked | 198.54–199.30 µs | 198.9 |
| BEVE selective, verify each read | whole value verified before selected paths | 882.38–886.20 µs | 884.2 |
| DannyPack custom reference full materialization | complete owned event | 2.0612–2.0850 ms | 2,072.5 |
| FlatBuffers selective, verify each read | whole buffer verified before selected fields | 2.1541–2.1608 ms | 2,157.4 |
| Cap’n Proto packed custom profile full materialization | complete owned event | 3.7338–3.7463 ms | 3,739.8 |
| Notepack custom reference full materialization | complete owned event | 3.7499–3.7600 ms | 3,754.8 |
| JSON full materialization | complete owned event | 4.2068–4.2229 ms | 4,214.6 |
| postcard reference full materialization | complete owned event | 4.4724–4.4844 ms | 4,478.2 |
| bincode reference full materialization | complete owned event | 4.5373–4.5513 ms | 4,544.1 |
| MessagePack full materialization | complete owned event | 4.6512–4.6689 ms | 4,659.7 |
| Thrift Compact full materialization | complete owned event | 4.9276–4.9445 ms | 4,935.7 |
| FlexBuffers full materialization | complete owned event | 5.2173–5.2298 ms | 5,223.5 |
| BEVE emerging reference full materialization | complete owned event | 5.8139–5.8310 ms | 5,822.3 |
| CBOR packed full materialization | complete owned event | 5.8543–5.8729 ms | 5,863.2 |
| Protocol Buffers binary full materialization | complete owned event | 5.9737–5.9917 ms | 5,982.4 |
| FlatBuffers full materialization | complete owned event | 6.1113–6.1446 ms | 6,126.8 |
| Avro binary datum full materialization | complete owned event | 6.6472–6.6792 ms | 6,660.7 |
| BSON full materialization | complete owned event | 10.4291–10.4603 ms | 10,444.3 |

Selective paths are not equivalent safety contracts. “Preverified” means the exact immutable buffer was fully verified earlier; that cost must be paid once at an ingress or trust boundary. Checked selected-field paths validate navigation, bounds, and selected types but may not validate skipped subtrees. Full materialization creates the complete owned event. The Cap’n Proto profile also contains custom packed byte blobs.
