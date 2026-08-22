//! Apache Avro binary datum encoding for Nostr events.
//!
//! Like Protobuf and FlatBuffers, this benchmark assumes the schema is already
//! known to both peers. It measures Avro's raw binary datum, not an Object
//! Container File or Single Object Encoding wrapper.

use std::io::Cursor;
use std::sync::OnceLock;

use apache_avro::reader::datum::GenericDatumReader;
use apache_avro::types::Value;
use apache_avro::writer::datum::GenericDatumWriter;
use apache_avro::Schema;

use crate::encoding::decode_lower_hex;
use crate::event::NostrEvent;

const EVENT_SCHEMA_JSON: &str = r#"
{
  "type": "record",
  "name": "NostrEvent",
  "namespace": "org.nostr",
  "fields": [
    {"name": "id", "type": {"type": "fixed", "name": "EventId", "size": 32}},
    {"name": "pubkey", "type": {"type": "fixed", "name": "Pubkey", "size": 32}},
    {"name": "created_at", "type": "long"},
    {"name": "kind", "type": "int"},
    {"name": "tags", "type": {"type": "array", "items": {"type": "array", "items": ["string", "bytes"]}}},
    {"name": "content", "type": "string"},
    {"name": "sig", "type": {"type": "fixed", "name": "Signature", "size": 64}}
  ]
}
"#;

const BATCH_SCHEMA_JSON: &str = r#"
{
  "type": "array",
  "items": {
    "type": "record",
    "name": "NostrEventBatchItem",
    "namespace": "org.nostr",
    "fields": [
      {"name": "id", "type": {"type": "fixed", "name": "BatchEventId", "size": 32}},
      {"name": "pubkey", "type": {"type": "fixed", "name": "BatchPubkey", "size": 32}},
      {"name": "created_at", "type": "long"},
      {"name": "kind", "type": "int"},
      {"name": "tags", "type": {"type": "array", "items": {"type": "array", "items": ["string", "bytes"]}}},
      {"name": "content", "type": "string"},
      {"name": "sig", "type": {"type": "fixed", "name": "BatchSignature", "size": 64}}
    ]
  }
}
"#;

fn event_schema() -> &'static Schema {
    static SCHEMA: OnceLock<Schema> = OnceLock::new();
    SCHEMA.get_or_init(|| Schema::parse_str(EVENT_SCHEMA_JSON).expect("valid Avro event schema"))
}

fn batch_schema() -> &'static Schema {
    static SCHEMA: OnceLock<Schema> = OnceLock::new();
    SCHEMA.get_or_init(|| Schema::parse_str(BATCH_SCHEMA_JSON).expect("valid Avro batch schema"))
}

fn event_writer() -> &'static GenericDatumWriter<'static> {
    static WRITER: OnceLock<GenericDatumWriter<'static>> = OnceLock::new();
    WRITER.get_or_init(|| {
        GenericDatumWriter::builder(event_schema())
            .build()
            .expect("valid Avro event writer")
    })
}

fn event_reader() -> &'static GenericDatumReader<'static> {
    static READER: OnceLock<GenericDatumReader<'static>> = OnceLock::new();
    READER.get_or_init(|| {
        GenericDatumReader::builder(event_schema())
            .build()
            .expect("valid Avro event reader")
    })
}

fn batch_writer() -> &'static GenericDatumWriter<'static> {
    static WRITER: OnceLock<GenericDatumWriter<'static>> = OnceLock::new();
    WRITER.get_or_init(|| {
        GenericDatumWriter::builder(batch_schema())
            .build()
            .expect("valid Avro batch writer")
    })
}

fn batch_reader() -> &'static GenericDatumReader<'static> {
    static READER: OnceLock<GenericDatumReader<'static>> = OnceLock::new();
    READER.get_or_init(|| {
        GenericDatumReader::builder(batch_schema())
            .build()
            .expect("valid Avro batch reader")
    })
}

fn event_to_value(event: &NostrEvent) -> Value {
    let tags = event
        .tags
        .iter()
        .map(|tag| {
            Value::Array(
                tag.iter()
                    .map(|value| match decode_lower_hex(value) {
                        Some(bytes) => Value::Union(1, Box::new(Value::Bytes(bytes))),
                        None => Value::Union(0, Box::new(Value::String(value.clone()))),
                    })
                    .collect(),
            )
        })
        .collect();

    Value::Record(vec![
        ("id".into(), Value::Fixed(32, event.id.to_vec())),
        ("pubkey".into(), Value::Fixed(32, event.pubkey.to_vec())),
        ("created_at".into(), Value::Long(event.created_at)),
        ("kind".into(), Value::Int(i32::from(event.kind))),
        ("tags".into(), Value::Array(tags)),
        ("content".into(), Value::String(event.content.clone())),
        ("sig".into(), Value::Fixed(64, event.sig.to_vec())),
    ])
}

fn take_field(fields: &mut Vec<(String, Value)>, name: &'static str) -> Result<Value, AvroError> {
    let index = fields
        .iter()
        .position(|(field, _)| field == name)
        .ok_or(AvroError::MissingField(name))?;
    Ok(fields.swap_remove(index).1)
}

fn fixed<const N: usize>(value: Value, name: &'static str) -> Result<[u8; N], AvroError> {
    match value {
        Value::Fixed(size, bytes) if size == N => {
            bytes.try_into().map_err(|_| AvroError::InvalidLength(name))
        }
        _ => Err(AvroError::InvalidType(name)),
    }
}

fn value_to_event(value: Value) -> Result<NostrEvent, AvroError> {
    let Value::Record(mut fields) = value else {
        return Err(AvroError::InvalidType("event"));
    };

    let tags = match take_field(&mut fields, "tags")? {
        Value::Array(tags) => tags
            .into_iter()
            .map(|tag| match tag {
                Value::Array(values) => values
                    .into_iter()
                    .map(|value| match value {
                        Value::Union(0, value) => match *value {
                            Value::String(text) => Ok(text),
                            _ => Err(AvroError::InvalidType("text tag value")),
                        },
                        Value::Union(1, value) => match *value {
                            Value::Bytes(bytes) => Ok(hex::encode(bytes)),
                            _ => Err(AvroError::InvalidType("binary tag value")),
                        },
                        _ => Err(AvroError::InvalidType("tag value")),
                    })
                    .collect(),
                _ => Err(AvroError::InvalidType("tag")),
            })
            .collect::<Result<Vec<Vec<String>>, AvroError>>()?,
        _ => return Err(AvroError::InvalidType("tags")),
    };

    let created_at = match take_field(&mut fields, "created_at")? {
        Value::Long(value) => value,
        _ => return Err(AvroError::InvalidType("created_at")),
    };
    let kind = match take_field(&mut fields, "kind")? {
        Value::Int(value) => u16::try_from(value).map_err(|_| AvroError::InvalidKind(value))?,
        _ => return Err(AvroError::InvalidType("kind")),
    };
    let content = match take_field(&mut fields, "content")? {
        Value::String(value) => value,
        _ => return Err(AvroError::InvalidType("content")),
    };

    Ok(NostrEvent {
        id: fixed(take_field(&mut fields, "id")?, "id")?,
        pubkey: fixed(take_field(&mut fields, "pubkey")?, "pubkey")?,
        created_at,
        kind,
        tags,
        content,
        sig: fixed(take_field(&mut fields, "sig")?, "sig")?,
    })
}

fn decode_value(data: &[u8], reader: &GenericDatumReader<'_>) -> Result<Value, AvroError> {
    let mut cursor = Cursor::new(data);
    let value = reader.read_value(&mut cursor)?;
    if cursor.position() != data.len() as u64 {
        return Err(AvroError::TrailingBytes(
            data.len() - cursor.position() as usize,
        ));
    }
    Ok(value)
}

pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    event_writer()
        .write_value_to_vec(event_to_value(event))
        .expect("Avro serialization should not fail")
}

pub fn deserialize(data: &[u8]) -> Result<NostrEvent, AvroError> {
    value_to_event(decode_value(data, event_reader())?)
}

pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    batch_writer()
        .write_value_to_vec(Value::Array(events.iter().map(event_to_value).collect()))
        .expect("Avro serialization should not fail")
}

pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, AvroError> {
    match decode_value(data, batch_reader())? {
        Value::Array(events) => events.into_iter().map(value_to_event).collect(),
        _ => Err(AvroError::InvalidType("batch")),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AvroError {
    #[error("Avro error: {0}")]
    Avro(#[from] apache_avro::Error),

    #[error("missing Avro field: {0}")]
    MissingField(&'static str),

    #[error("invalid Avro type for {0}")]
    InvalidType(&'static str),

    #[error("invalid {0} length")]
    InvalidLength(&'static str),

    #[error("Avro kind is outside the Nostr u16 range: {0}")]
    InvalidKind(i32),

    #[error("Avro input has {0} trailing bytes")]
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
