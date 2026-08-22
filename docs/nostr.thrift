namespace * org.nostr

// Exactly one of text or hex is written. `hex` represents the decoded bytes
// of an eligible lowercase hexadecimal NIP-01 tag value.
struct TagValue {
  1: optional string text,
  2: optional binary hex,
}

struct Tag {
  1: required list<TagValue> values,
}

struct NostrEvent {
  1: required binary id,
  2: required binary pubkey,
  3: required i64 created_at,
  4: required i16 kind,
  5: required list<Tag> tags,
  6: required string content,
  7: required binary sig,
}

struct EventBatch {
  1: required list<NostrEvent> events,
}
