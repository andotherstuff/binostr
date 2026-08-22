//! BSON 1.1 document encoding for Nostr events.
//!
//! BSON is included as a widely deployed, schema-less binary document format.
//! Field names and BSON array indexes are part of its standard representation,
//! so their size cost is intentionally retained.

use std::io::Cursor;

use ::bson::spec::BinarySubtype;
use ::bson::{Binary, Bson, Document};

use crate::encoding::decode_lower_hex;
use crate::event::NostrEvent;

fn binary(bytes: &[u8]) -> Bson {
    Bson::Binary(Binary {
        subtype: BinarySubtype::Generic,
        bytes: bytes.to_vec(),
    })
}

fn event_to_document(event: &NostrEvent) -> Document {
    let tags = event
        .tags
        .iter()
        .map(|tag| {
            Bson::Array(
                tag.iter()
                    .map(|value| match decode_lower_hex(value) {
                        Some(bytes) => binary(&bytes),
                        None => Bson::String(value.clone()),
                    })
                    .collect(),
            )
        })
        .collect();

    Document::from_iter([
        ("id".into(), binary(&event.id)),
        ("pubkey".into(), binary(&event.pubkey)),
        ("created_at".into(), Bson::Int64(event.created_at)),
        ("kind".into(), Bson::Int32(i32::from(event.kind))),
        ("tags".into(), Bson::Array(tags)),
        ("content".into(), Bson::String(event.content.clone())),
        ("sig".into(), binary(&event.sig)),
    ])
}

fn binary_field<const N: usize>(
    document: &Document,
    name: &'static str,
) -> Result<[u8; N], BsonError> {
    document
        .get_binary_generic(name)?
        .as_slice()
        .try_into()
        .map_err(|_| BsonError::InvalidLength(name))
}

fn document_to_event(document: &Document) -> Result<NostrEvent, BsonError> {
    let tags = document
        .get_array("tags")?
        .iter()
        .map(|tag| {
            tag.as_array()
                .ok_or(BsonError::InvalidType("tag"))?
                .iter()
                .map(|value| match value {
                    Bson::String(text) => Ok(text.clone()),
                    Bson::Binary(binary) if binary.subtype == BinarySubtype::Generic => {
                        Ok(hex::encode(&binary.bytes))
                    }
                    _ => Err(BsonError::InvalidType("tag value")),
                })
                .collect()
        })
        .collect::<Result<Vec<Vec<String>>, BsonError>>()?;

    let kind = document.get_i32("kind")?;
    Ok(NostrEvent {
        id: binary_field(document, "id")?,
        pubkey: binary_field(document, "pubkey")?,
        created_at: document.get_i64("created_at")?,
        kind: u16::try_from(kind).map_err(|_| BsonError::InvalidKind(kind))?,
        tags,
        content: document.get_str("content")?.to_owned(),
        sig: binary_field(document, "sig")?,
    })
}

fn parse_document(data: &[u8]) -> Result<Document, BsonError> {
    let mut cursor = Cursor::new(data);
    let document = Document::from_reader(&mut cursor)?;
    if cursor.position() != data.len() as u64 {
        return Err(BsonError::TrailingBytes(
            data.len() - cursor.position() as usize,
        ));
    }
    Ok(document)
}

pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    event_to_document(event)
        .to_vec()
        .expect("BSON serialization should not fail")
}

pub fn deserialize(data: &[u8]) -> Result<NostrEvent, BsonError> {
    document_to_event(&parse_document(data)?)
}

pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    Document::from_iter([(
        "events".into(),
        Bson::Array(
            events
                .iter()
                .map(|event| Bson::Document(event_to_document(event)))
                .collect(),
        ),
    )])
    .to_vec()
    .expect("BSON serialization should not fail")
}

pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, BsonError> {
    parse_document(data)?
        .get_array("events")?
        .iter()
        .map(|value| {
            document_to_event(
                value
                    .as_document()
                    .ok_or(BsonError::InvalidType("batch event"))?,
            )
        })
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum BsonError {
    #[error("BSON error: {0}")]
    Bson(#[from] ::bson::error::Error),

    #[error("invalid BSON type for {0}")]
    InvalidType(&'static str),

    #[error("invalid {0} length")]
    InvalidLength(&'static str),

    #[error("BSON kind is outside the Nostr u16 range: {0}")]
    InvalidKind(i32),

    #[error("BSON input has {0} trailing bytes")]
    TrailingBytes(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> NostrEvent {
        NostrEvent {
            id: [1; 32],
            pubkey: [2; 32],
            created_at: 1_700_000_000,
            kind: 1,
            tags: vec![
                vec!["e".into(), hex::encode([3; 32])],
                vec!["custom".into(), "DEADBEEF".into()],
            ],
            content: "hello".into(),
            sig: [4; 64],
        }
    }

    #[test]
    fn roundtrip() {
        let event = event();
        assert_eq!(deserialize(&serialize(&event)).unwrap(), event);
    }

    #[test]
    fn batch_roundtrip() {
        let events = vec![event(), event()];
        assert_eq!(
            deserialize_batch(&serialize_batch(&events)).unwrap(),
            events
        );
    }
}
