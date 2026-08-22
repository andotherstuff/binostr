# Security policy

Binostr is a research and benchmarking repository, not a production-ready Nostr SDK. Its decoders process untrusted-format inputs in tests, but they have not received a complete security audit or continuous coverage-guided fuzzing.

## Reporting a vulnerability

Please use GitHub’s private vulnerability reporting for `andotherstuff/binostr` when available. Otherwise contact the repository owner privately before opening a public issue. Include the affected codec/profile, commit, reproducer, impact, and whether the input crosses a trust boundary.

Do not include private keys, non-public events, or sensitive relay captures in a report.

## Supported versions

Only the current `master` branch is maintained. There are no supported releases yet.

## Production use

Before using this code on an untrusted network, add deployment-specific wire and structural limits, fuzzing, dependency review, concurrency/load testing, and an independent security review. The `deserialize_limited` helper provides an outer size bound and post-decode structural checks; individual third-party codecs may allocate while decoding before post-decode limits can run.
