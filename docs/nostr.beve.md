# Nostr BEVE benchmark profile

BEVE is measured as an emerging performance reference, not as a recommended interoperable SDK wire format. Its open Version 2 data model can express the same positional event profile as CBOR and MessagePack, but the project has not demonstrated the broad Swift, Kotlin, JavaScript/TypeScript, and server-language support required by this repository's standards-first selection policy.

An event is a seven-element array in NIP-01 field order. ID, public key, signature, and eligible lowercase hexadecimal tag values use unsigned-byte typed arrays. Other tag values and content use UTF-8 strings. Integers retain signed `created_at` and the complete `u16` kind range. A batch is an array of event arrays.

BEVE typed byte arrays arrive through Serde as sequences during type-erased tag decoding, so the BEVE adapter has a format-specific tag visitor. The encoder otherwise uses the same shared positional model as CBOR, MessagePack, and FlexBuffers.

No independent non-Rust vector is claimed. A broad-language implementation and maintenance audit is required before BEVE can move from an emerging reference into the interoperable-candidate class.
