# Nostr FlexBuffers profile

This profile uses the standard FlexBuffers data model and the official Rust runtime. It intentionally matches the packed CBOR and MessagePack semantic shape so differences measure the codecs rather than different event mappings.

## Event representation

An event is a seven-element vector in NIP-01 field order:

```text
[
  id:         blob of length 32,
  pubkey:     blob of length 32,
  created_at: signed integer,
  kind:       unsigned integer in 0...65535,
  tags:       vector<tag>,
  content:    string,
  sig:        blob of length 64
]
```

Each tag is a vector. A tag value is either unchanged text or a blob containing the decoded bytes of a lowercase, even-length hexadecimal NIP-01 string of at least eight characters. On decode, blobs in tag position become lowercase hexadecimal text. All other tag strings remain text, making the transform reversible.

## Determinism and evolution

The base format permits multiple valid physical layouts through width choices and deduplication. The bytes in `vectors/interop-v1.json` are deterministic output from the pinned official runtime, but they are not claimed to be a universal canonical FlexBuffers representation.

A batch is a vector of event vectors. The event vector has exactly seven elements; adding fields requires a negotiated profile version. New Nostr kinds and tag conventions require no profile change.

Nostr IDs and signatures still use NIP-01 canonical JSON. This is only a transport or storage representation.
