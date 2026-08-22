# Binostr benchmark snapshot

Generated `2026-08-21T22:14:26Z` from commit `0251b60ab9aea1d804a45b5f14ec62a46ff651b3` (dirty tree) using `rustc 1.96.1 (31fca3adb 2026-06-26) (Homebrew)`. Corpus: 1000 events, seed `4776434982410539521`, ID fingerprint `a05ceb17de26aca15c5b284652a6e55188b0c5a135fb89839459d9d93ae492f5`.

Times are nanoseconds per event. Tail latency uses chunks of 20 events to reduce timer noise. Allocation byte counts include reallocations.

| Format | Class | Encode | Decode | Decode + ID | Fully validated | Bytes | Zstd stream | Decode allocs | Decode allocated bytes | Decode p99 |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| JSON | nostr-baseline | 3215 | 4284 | 9770 | 28173 | 2618.4 | 1031681 | 76.0 | 7524 | 16989 |
| CBOR Packed | interoperable-candidate | 6426 | 5864 | 11344 | 29561 | 1685.7 | 953641 | 92.2 | 5221 | 37558 |
| MessagePack | interoperable-candidate | 6552 | 4732 | 10273 | 28491 | 1685.6 | 955913 | 92.2 | 4797 | 30752 |
| FlatBuffers | interoperable-candidate | 8440 | 6154 | 11779 | 30069 | 2906.9 | 1116188 | 69.6 | 5877 | 38741 |
| FlexBuffers | interoperable-candidate | 7672 | 5212 | 10832 | 28920 | 1898.5 | 1016734 | 92.2 | 4797 | 33670 |
| Avro Binary Datum | interoperable-candidate | 10251 | 6661 | 12327 | 30584 | 1730.1 | 948660 | 149.6 | 10034 | 43577 |
| BSON | interoperable-candidate | 10659 | 10385 | 16054 | 34365 | 2275.2 | 990576 | 176.1 | 31161 | 62787 |
| Thrift Compact | interoperable-candidate | 8255 | 4986 | 10449 | 28547 | 1811.0 | 953276 | 93.2 | 4734 | 32735 |
| BEVE | emerging-reference | 8133 | 5897 | 11389 | 29796 | 1740.7 | 950201 | 92.2 | 4797 | 38566 |
| bincode | rust-reference | 6480 | 4587 | 10274 | 28405 | 1707.0 | 949118 | 89.2 | 4669 | 30768 |
| postcard | rust-reference | 6307 | 4567 | 11914 | 28386 | 1706.6 | 947952 | 89.2 | 4669 | 30485 |
| Proto Binary | interoperable-candidate | 7703 | 6018 | 11558 | 29640 | 1835.4 | 957939 | 114.9 | 9545 | 37977 |
| Cap'n Packed | standard-envelope-custom-profile | 7404 | 3853 | 9331 | 27917 | 1728.5 | 996783 | 71.6 | 5748 | 23897 |
| DannyPack | custom-reference | 492 | 2098 | 7627 | 25829 | 1662.7 | 952623 | 69.6 | 4155 | 12897 |
| Notepack | custom-reference | 721 | 3876 | 9397 | 27475 | 1658.4 | 951254 | 69.6 | 5878 | 24850 |

## Interpretation guardrails

- Results compare these Rust implementations and profiles, not abstract format ceilings.
- `Fully validated` includes bounded decode, structural checks, NIP-01 ID recomputation, and BIP-340 verification.
- Compressed streams use the same four-byte big-endian record framing for every format. Native batch sizes and timings are retained in the JSON artifact.
- Interoperability and ecosystem support are protocol-selection criteria that timings cannot capture.
