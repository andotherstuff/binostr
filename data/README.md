# Benchmark corpus governance

This directory contains a tracked sample of real Nostr events. It is useful for exercising realistic content, tag, and kind distributions, but it is **not yet suitable evidence for claims about the Nostr network as a whole**.

## Verified facts

- `sample.pb.gz` contains 50,000 length-delimited Protobuf events.
- All 50,000 event IDs are unique.
- All event IDs recompute correctly and all BIP-340 signatures verify.
- The compressed file is 58,204,090 bytes with SHA-256 `6782937ad398d79f9154851d4de95a6c3bd9bed570a301b5019df8606cd18292`.
- The complete machine-readable audit, including kind distribution and extrema, is in [`../results/corpus-audit.json`](../results/corpus-audit.json).

These checks establish integrity of the currently tracked bytes. They do not establish provenance, representativeness, consent, or redistribution rights.

## Missing provenance

Repository history describes the sample as eight days of real traffic, but does not identify the relay or source, query/filter, collection dates, collector version, transformations, sampling policy, license or redistribution basis, or privacy review. Those facts are therefore unknown. The derived JSONL files in this directory do not repair that gap.

Do not cite results from this corpus as a formal network-wide study. The repository's code is MIT-licensed; that code license does not grant rights in the event corpus.

## Requirements for replacement or formal publication

A replacement corpus must include:

1. source relays or dataset and exact collection interval;
2. capture query, sampling procedure, collector version, and reproducible transformation commands;
3. content hash and a cryptographic validity audit;
4. license or other redistribution basis and a privacy/ethics review;
5. documented redactions or exclusions; and
6. an explicit statement of what population the sample can and cannot represent.

Removing the current files from a future revision would not remove them from Git history. Any decision to rewrite published history, replace the corpus, or continue redistributing it requires an explicit maintainer decision after the missing provenance is investigated.
