# Nostr deterministic CBOR profile

This profile maps a NIP-01 event onto the standard CBOR data model from RFC 8949. The normative data shape is `nostr-event-packed` in `nostr.cddl`; this document fixes choices that CDDL alone does not express.

## Event representation

An event is a seven-element array in NIP-01 field order:

```text
[
  id:         byte string of length 32,
  pubkey:     byte string of length 32,
  created_at: integer,
  kind:       unsigned integer in 0...65535,
  tags:       array<tag>,
  content:    text string,
  sig:        byte string of length 64
]
```

Each tag is an array. A tag value is either unchanged text or the decoded bytes of a lowercase, even-length hexadecimal NIP-01 string of at least eight characters. On decode, a byte-string tag value becomes lowercase hexadecimal text. Uppercase, mixed-case, odd-length, short, and non-hex strings remain text, making the transform reversible.

## Deterministic encoding

Conforming deterministic encoders:

1. use the field and array order above;
2. use definite-length arrays, text strings, and byte strings;
3. use RFC 8949 preferred serialization for integers and lengths;
4. encode every eligible tag value as a byte string;
5. emit no CBOR tags, floating-point values, maps, or trailing data; and
6. encode text as valid UTF-8 without normalization.

Decoders must enforce the fixed cryptographic-field lengths and kind range. Resource limits, framing, and whether non-preferred but semantically valid CBOR is accepted are protocol-boundary policies; an SDK should expose strict and compatibility modes explicitly rather than silently treating them as the same contract.

Nostr event IDs and signatures continue to use NIP-01 canonical JSON. This profile does not redefine event identity or signing.

## Batch, framing, and evolution

A batch is a definite-length array of event arrays. A streaming transport may instead length-frame individual events; it must specify its maximum frame size and whether multiple CBOR items are permitted in one frame.

The seven-element array is deliberately exact. Adding an eighth field changes the profile and requires an independently negotiated version. New Nostr event kinds and tag conventions do not require a wire-profile change because `kind`, `tags`, and `content` already carry them.
