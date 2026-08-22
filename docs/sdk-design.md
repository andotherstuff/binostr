# Binary Nostr SDK design targets

## Optimize the accepted-event pipeline

The top-level performance objective should be **validated, policy-usable events per second under explicit memory bounds**. A relay or client rarely benefits from a decoded object that has not passed size limits, structural checks, NIP-01 ID recomputation, signature verification, and local policy.

Raw deserialization is still worth measuring because routing, cache reads, trusted local storage, and repeated filtering may skip some stages. It is not a sufficient proxy for ingress cost.

## Architecture

Separate these layers:

1. **Framing** finds a complete record, enforces a wire-size limit before allocation, and distinguishes incomplete input from invalid input.
2. **Codec** verifies the format and exposes a borrowed event view where the format and language safely permit it.
3. **Semantic validation** checks fixed lengths, UTF-8, integer ranges, tag/value counts, content size, NIP-01 ID, and BIP-340 signature.
4. **Policy** applies relay/client rules such as timestamps, kinds, authorization, duplicate handling, and storage limits.
5. **Ownership boundary** materializes only fields that must outlive the input buffer or be mutated.

The public API should make those costs visible. Suggested shapes are `FrameDecoder`, `EventView`, `ValidatedEventView`, `OwnedEvent`, and a `Limits` object, with explicit `to_owned()` rather than hidden copying.

## Operations to optimize

- **Ingress:** bounded frame decode plus complete validation.
- **Relay filtering:** kind, pubkey, ID, timestamp, and first tag values without decoding content or all tags where possible.
- **Fan-out:** validate once, then reuse immutable wire bytes or a validated view across subscribers.
- **Storage:** length-framed records with an explicit codec/profile version; do not rely on concatenation behavior.
- **Client reads:** lazy content/tag materialization for lists and notifications; owned conversion for durable application state.
- **Batches:** cap both event count and aggregate bytes; stream validation so one batch does not require two complete owned copies.

## Correctness and security properties

- NIP-01 JSON remains the signing and event-ID representation regardless of transport encoding.
- Arbitrary tag strings are lossless. Binary compaction must be type-tagged and reversible; never normalize case or guess semantics from tag names.
- Reject duplicate required fields in map/field-number formats unless the profile explicitly defines a safe rule.
- Define unknown-field behavior and a profile/version negotiation mechanism before deployment.
- Apply byte, nesting, collection-count, and string limits before or during allocation—not only after full decode.
- Treat zero-copy access as unsafe until the complete buffer has passed the format verifier; tie borrowed views to the immutable buffer lifetime.
- Preserve clear `Incomplete`, `Malformed`, `LimitExceeded`, `InvalidId`, `InvalidSignature`, and `UnsupportedProfile` errors.

## Decision data still needed for a production SDK

The repository now measures the main local codec dimensions, but the final choice should also use:

- target-language implementation and maintenance audits, especially Swift and Kotlin;
- WebSocket and relay experiments using complete protocol messages and realistic concurrency;
- peak RSS and retained-memory measurements in addition to allocator request counts;
- coverage-guided fuzzing and differential decode/re-encode tests;
- schema/profile evolution drills with old readers and new writers; and
- operational traces showing the actual proportions of ingress, filtering, fan-out, cache reads, archival batches, and client UI reads.

If selective reads are rare and every accepted event must be cryptographically validated, prioritize mature libraries, predictable limits, and interoperability over shaving a few hundred nanoseconds from owned decode. If relays repeatedly filter already-validated immutable buffers, a verified borrowed representation can matter much more.
