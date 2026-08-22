//! Protocol Buffers serialization
//!
//! Two variants:
//! 1. String - uses hex strings for id/pubkey/sig (compatible with existing schema)
//! 2. Binary - uses raw bytes for id/pubkey/sig (optimized for size)

use prost::Message;

use crate::encoding::decode_lower_hex;
use crate::event::NostrEvent;
use crate::proto_gen::nostr::{ProtoEvent, Tag};
use crate::proto_gen::nostr_binary::{
    tag_value_binary, ProtoEventBinary, TagBinary, TagValueBinary,
};

// ============================================
// Variant 1: String (hex-encoded)
// ============================================

pub mod string {
    use super::*;

    pub fn serialize(event: &NostrEvent) -> Vec<u8> {
        let proto = event_to_proto(event);
        proto.encode_to_vec()
    }

    pub fn deserialize(data: &[u8]) -> Result<NostrEvent, ProtoError> {
        let proto = ProtoEvent::decode(data)?;
        proto_to_event(proto)
    }

    pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
        use crate::proto_gen::nostr::EventBatch;

        let batch = EventBatch {
            events: events.iter().map(event_to_proto).collect(),
        };
        batch.encode_to_vec()
    }

    pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, ProtoError> {
        use crate::proto_gen::nostr::EventBatch;

        let batch = EventBatch::decode(data)?;
        batch.events.into_iter().map(proto_to_event).collect()
    }

    fn event_to_proto(event: &NostrEvent) -> ProtoEvent {
        ProtoEvent {
            id: event.id_hex(),
            pubkey: event.pubkey_hex(),
            created_at: event.created_at,
            kind: event.kind as i32,
            tags: event
                .tags
                .iter()
                .map(|t| Tag { values: t.clone() })
                .collect(),
            content: event.content.clone(),
            sig: event.sig_hex(),
        }
    }

    fn proto_to_event(proto: ProtoEvent) -> Result<NostrEvent, ProtoError> {
        let id = hex::decode(&proto.id)?;
        let pubkey = hex::decode(&proto.pubkey)?;
        let sig = hex::decode(&proto.sig)?;
        let kind = u16::try_from(proto.kind).map_err(|_| ProtoError::InvalidKind(proto.kind))?;

        Ok(NostrEvent {
            id: id.try_into().map_err(|_| ProtoError::InvalidLength("id"))?,
            pubkey: pubkey
                .try_into()
                .map_err(|_| ProtoError::InvalidLength("pubkey"))?,
            created_at: proto.created_at,
            kind,
            tags: proto.tags.into_iter().map(|t| t.values).collect(),
            content: proto.content,
            sig: sig
                .try_into()
                .map_err(|_| ProtoError::InvalidLength("sig"))?,
        })
    }
}

// ============================================
// Variant 2: Binary (raw bytes)
// ============================================

pub mod binary {
    use super::*;

    pub fn serialize(event: &NostrEvent) -> Vec<u8> {
        let proto = event_to_proto_binary(event);
        proto.encode_to_vec()
    }

    pub fn deserialize(data: &[u8]) -> Result<NostrEvent, ProtoError> {
        let proto = ProtoEventBinary::decode(data)?;
        proto_binary_to_event(proto)
    }

    pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
        use crate::proto_gen::nostr_binary::EventBatchBinary;

        let batch = EventBatchBinary {
            events: events.iter().map(event_to_proto_binary).collect(),
        };
        batch.encode_to_vec()
    }

    pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, ProtoError> {
        use crate::proto_gen::nostr_binary::EventBatchBinary;

        let batch = EventBatchBinary::decode(data)?;
        batch
            .events
            .into_iter()
            .map(proto_binary_to_event)
            .collect()
    }

    fn event_to_proto_binary(event: &NostrEvent) -> ProtoEventBinary {
        ProtoEventBinary {
            id: event.id.to_vec(),
            pubkey: event.pubkey.to_vec(),
            created_at: event.created_at,
            kind: event.kind as i32,
            tags: event
                .tags
                .iter()
                .map(|tag| TagBinary {
                    values: Vec::new(),
                    values_v2: tag
                        .iter()
                        .map(|value| TagValueBinary {
                            value: Some(match decode_lower_hex(value) {
                                Some(bytes) => tag_value_binary::Value::Hex(bytes),
                                None => tag_value_binary::Value::Text(value.clone()),
                            }),
                        })
                        .collect(),
                })
                .collect(),
            content: event.content.clone(),
            sig: event.sig.to_vec(),
        }
    }

    fn proto_binary_to_event(proto: ProtoEventBinary) -> Result<NostrEvent, ProtoError> {
        let kind = u16::try_from(proto.kind).map_err(|_| ProtoError::InvalidKind(proto.kind))?;
        Ok(NostrEvent {
            id: proto
                .id
                .try_into()
                .map_err(|_| ProtoError::InvalidLength("id"))?,
            pubkey: proto
                .pubkey
                .try_into()
                .map_err(|_| ProtoError::InvalidLength("pubkey"))?,
            created_at: proto.created_at,
            kind,
            tags: proto
                .tags
                .into_iter()
                .map(|tag| {
                    if !tag.values.is_empty() && !tag.values_v2.is_empty() {
                        return Err(ProtoError::ConflictingTagEncoding);
                    }
                    if tag.values_v2.is_empty() {
                        return Ok(tag.values);
                    }

                    tag.values_v2
                        .into_iter()
                        .map(|value| match value.value {
                            Some(tag_value_binary::Value::Text(text)) => Ok(text),
                            Some(tag_value_binary::Value::Hex(bytes)) => Ok(hex::encode(bytes)),
                            None => Err(ProtoError::MissingValue("tag value")),
                        })
                        .collect()
                })
                .collect::<Result<Vec<Vec<String>>, ProtoError>>()?,
            content: proto.content,
            sig: proto
                .sig
                .try_into()
                .map_err(|_| ProtoError::InvalidLength("sig"))?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("Protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),

    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("Invalid length for field: {0}")]
    InvalidLength(&'static str),

    #[error("Missing {0}")]
    MissingValue(&'static str),

    #[error("Protobuf kind is outside the Nostr u16 range: {0}")]
    InvalidKind(i32),

    #[error("Protobuf tag contains both legacy and binary-aware values")]
    ConflictingTagEncoding,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> NostrEvent {
        NostrEvent {
            id: [0xab; 32],
            pubkey: [0xcd; 32],
            created_at: 1234567890,
            kind: 1,
            tags: vec![
                vec!["p".to_string(), "abc123".to_string()],
                vec!["e".to_string(), "def456".to_string()],
            ],
            content: "Hello, Nostr!".to_string(),
            sig: [0xef; 64],
        }
    }

    #[test]
    fn test_string_roundtrip() {
        let event = sample_event();
        let bytes = string::serialize(&event);
        let back = string::deserialize(&bytes).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_binary_roundtrip() {
        let event = sample_event();
        let bytes = binary::serialize(&event);
        let back = binary::deserialize(&bytes).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn binary_decoder_accepts_legacy_text_tags() {
        let event = sample_event();
        let proto = ProtoEventBinary {
            id: event.id.to_vec(),
            pubkey: event.pubkey.to_vec(),
            created_at: event.created_at,
            kind: event.kind.into(),
            tags: event
                .tags
                .iter()
                .map(|tag| TagBinary {
                    values: tag.clone(),
                    values_v2: Vec::new(),
                })
                .collect(),
            content: event.content.clone(),
            sig: event.sig.to_vec(),
        };

        assert_eq!(binary::deserialize(&proto.encode_to_vec()).unwrap(), event);
    }

    #[test]
    fn binary_decoder_rejects_out_of_range_kind_and_ambiguous_tags() {
        let event = sample_event();
        let mut proto = ProtoEventBinary {
            id: event.id.to_vec(),
            pubkey: event.pubkey.to_vec(),
            created_at: event.created_at,
            kind: -1,
            tags: vec![],
            content: event.content.clone(),
            sig: event.sig.to_vec(),
        };
        assert!(matches!(
            binary::deserialize(&proto.encode_to_vec()),
            Err(ProtoError::InvalidKind(-1))
        ));

        proto.kind = 1;
        proto.tags.push(TagBinary {
            values: vec!["legacy".into()],
            values_v2: vec![TagValueBinary {
                value: Some(tag_value_binary::Value::Text("new".into())),
            }],
        });
        assert!(matches!(
            binary::deserialize(&proto.encode_to_vec()),
            Err(ProtoError::ConflictingTagEncoding)
        ));
    }

    #[test]
    fn test_size_comparison() {
        let event = sample_event();

        let string_size = string::serialize(&event).len();
        let binary_size = binary::serialize(&event).len();

        println!("Proto String: {} bytes", string_size);
        println!("Proto Binary: {} bytes", binary_size);

        // Binary should be significantly smaller (saves 128 bytes of hex overhead)
        assert!(binary_size < string_size);
    }

    #[test]
    fn test_batch_roundtrip() {
        let events = vec![sample_event(), sample_event()];

        // String batch
        let bytes = string::serialize_batch(&events);
        let back = string::deserialize_batch(&bytes).unwrap();
        assert_eq!(events, back);

        // Binary batch
        let bytes = binary::serialize_batch(&events);
        let back = binary::deserialize_batch(&bytes).unwrap();
        assert_eq!(events, back);
    }
}
