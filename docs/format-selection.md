# Format selection

## Standards-first policy

An SDK default should use a documented, royalty-free format with multiple maintained implementations and viable libraries in the SDK’s target languages. A Nostr-specific codec remains useful as a performance reference, but it should replace an open format only after demonstrating a material end-to-end advantage after framing, validation, allocations, operational debugging, evolution, and interoperability are counted.

The binary profile itself is part of the protocol. Saying “CBOR” or “MessagePack” is insufficient without defining field order, integer rules, text-versus-bytes behavior, extension handling, batch representation, framing, limits, and canonical output.

## Measured set

| Format/profile | Governance and ecosystem | Role here | Important caveat |
|---|---|---|---|
| NIP-01 JSON | Existing Nostr baseline | Required compatibility baseline | Larger raw records; still required for event ID computation |
| CBOR packed | IETF [RFC 8949](https://www.rfc-editor.org/rfc/rfc8949); broad implementations | Leading standards-first candidate | The positional Nostr profile and deterministic rules must be standardized |
| MessagePack | Open [format specification](https://github.com/msgpack/msgpack/blob/master/spec.md); project lists 50+ language environments | Leading broadly supported candidate | No IETF-style standards body; the Nostr profile carries evolution rules |
| Protocol Buffers binary | Open specification and very broad generated tooling | Schema-evolution candidate | Requires schema/code generation; unknown-field and canonical-output policies matter |
| FlatBuffers | Open schema format with many language implementations and verified zero-copy access in some runtimes | Selective-read candidate | This schema is much larger on the measured Nostr distribution; verifier support is language-dependent |
| FlexBuffers | Official schema-less format in the FlatBuffers project; current Rust and Python implementations | Requested schema-less/borrowed-format comparison | Narrower language and verifier story than schema-based FlatBuffers; the base format is not canonically encoded |
| Apache Avro binary datum | Apache project; mature multi-language data ecosystem | Compact schema-driven candidate | Raw datum has no self-identifying schema or record boundary; schema negotiation and framing are mandatory |
| Apache Thrift Compact | Apache project with broad cross-language code generation | Compact schema-driven candidate | Compact Protocol support varies across the language matrix; `kind` uses the `i16` bit pattern to preserve `u16` |
| BSON 1.1 | Open specification and widely deployed document libraries | Frequently requested document-format comparison | Encoded field names and array indexes add cost; weaker fit for a fixed seven-field message |
| BEVE Version 2 | Published open format with current C++, Rust, and Go implementations | Emerging performance reference | Not yet a broad-language SDK candidate; no independent non-Rust vector is claimed here |
| Cap’n Proto packed | Open schema/runtime ecosystem | Selective-read and packed-size comparison | This repository puts fixed fields and tags inside custom byte blobs; it is a standard envelope around a custom Nostr profile, not an off-the-shelf schema mapping |
| bincode, postcard | Well-used Rust codecs | Rust implementation references | Not cross-language wire-standard candidates |
| DannyPack, Notepack | Nostr-specific implementations | Custom performance references | Bespoke ecosystem, review, evolution, and maintenance burden |

Every measured interoperable standard/envelope profile has a deterministic Rust vector decoded by an independent Python implementation, now including FlexBuffers. BEVE is explicitly an emerging reference and is excluded from that claim. A successful fixture demonstrates semantic interoperability for these bytes, not universal compatibility or canonical re-encoding across every library.

## Formats considered but not separately benchmarked

| Format | Disposition |
|---|---|
| DAG-CBOR | A constrained CBOR application profile; evaluate its constraints when designing the CBOR profile rather than presenting it as a different base codec |
| Amazon Ion | Rich typed document model and mature implementations in a smaller set of ecosystems; extra types do not solve a Nostr requirement demonstrated here |
| Smile | Primarily associated with the Jackson ecosystem; weaker broad-language choice than CBOR or MessagePack |
| UBJSON | Open Apache-licensed specification with many listed language libraries, but the Rust implementations inspected were not suitable for a fair reversible benchmark: one only serializes, while another leaves byte serialization/deserialization unimplemented. Revisit when a maintained complete Rust implementation exists |
| BJData | UBJSON-derived format aimed at scientific and binary array data; weaker client-SDK ecosystem fit than the measured general-purpose candidates |
| ASN.1 PER/OER/DER | Extremely mature standards family, but choosing encoding rules and building comparable schemas/toolchains is a separate protocol project; revisit for deployments already committed to ASN.1 |
| XDR | Open IETF encoding with stable tooling, but its rigid/aligned model is a poor expected size fit for nested variable strings and adds little beyond the schema candidates already measured |
| Simple Binary Encoding | Excellent for fixed, latency-sensitive market-data messages; Nostr’s unbounded content and nested tag arrays are a poor workload match |
| Apache Arrow | Columnar in-memory/batch analytics representation, not a single-event client-relay message format |
| FlatBuffers object API variants | Implementation strategies for the same wire profile, not additional formats; selective and fully owned reads are measured separately |
| rkyv | Important Rust zero-copy archive format, but not a cross-language open wire-standard candidate; FlatBuffers and Cap’n Proto already exercise the relevant borrowed-access workload |

Add one of these when a concrete target-language, deployment, or compatibility requirement makes it credible. Merely increasing the format count makes the comparison less fair if the implementation is immature or the mapping is contrived.

## Current recommendation

Keep the SDK’s semantic event model and validation pipeline independent of its codec. For an open default, take **deterministic packed CBOR** into protocol-design work first because it has IETF governance, broad implementation availability, competitive size, and a simple data model. Carry **MessagePack** as the closest alternative: with the same typed positional model and allocation count, this Rust implementation decodes MessagePack about 21% faster while producing essentially the same raw size. CBOR requests modestly more decode-allocation bytes on this corpus. Use the actual target-language matrix—especially Swift, Kotlin, JavaScript/TypeScript, Rust, and server languages—to decide between them.

Protocol Buffers remains the strongest alternative when explicit generated schemas and evolution discipline are desired. Cap’n Proto, FlatBuffers, or FlexBuffers should be selected only if borrowed selective access proves important in the real SDK API and target languages; generic owned decode benchmarks do not capture their intended advantage. BEVE remains a performance reference until independent target-language support is demonstrated.

Do not select the wire format from raw deserialize time alone. In the current report, signature verification and ID recomputation dominate enough that the spread between fully validated candidates is far smaller than the spread between their raw decoders.
