# Nostr MessagePack profile

This profile encodes a NIP-01 event with the standard [MessagePack](https://msgpack.org/) data model. It intentionally uses only arrays, strings, binary values, and integers so implementations do not need extension types or project-specific codecs.

## Event representation

An event is a seven-element array in NIP-01 field order:

```text
[
  id:         bin 32,
  pubkey:     bin 32,
  created_at: integer,
  kind:       unsigned integer,
  tags:       array<tag>,
  content:    string,
  sig:        bin 64
]
```

Each tag is an array of tag values. A tag value is either:

- a MessagePack string, representing the NIP-01 string unchanged; or
- a MessagePack binary value, representing the decoded bytes of a lowercase, even-length hexadecimal NIP-01 string of at least eight characters.

On decode, a binary tag value is converted to lowercase hexadecimal text. Uppercase and short hexadecimal-looking strings must remain MessagePack strings. This rule keeps the conversion reversible and avoids changing arbitrary tag data.

## Deterministic profile

When deterministic output is required, encoders must:

1. use the field and array ordering above;
2. encode eligible lowercase hexadecimal tag values as binary values;
3. use MessagePack's shortest representation for each integer and collection length; and
4. emit no extension types or trailing values.

Nostr event IDs and signatures are still defined by NIP-01's canonical JSON serialization. This profile is a transport/storage representation and does not redefine event identity or signing.

## Batch representation

A batch is a MessagePack array of event arrays. Streaming transports may instead frame individual event values; framing is a transport concern and must be specified separately.
