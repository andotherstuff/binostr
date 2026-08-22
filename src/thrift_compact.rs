//! Apache Thrift Compact Protocol encoding for Nostr events.
//!
//! The implementation follows `docs/nostr.thrift` directly using Thrift's
//! protocol API, avoiding a build-time dependency on the Thrift compiler.

use std::io::Cursor;

use thrift::protocol::{
    TCompactInputProtocol, TCompactOutputProtocol, TFieldIdentifier, TInputProtocol,
    TListIdentifier, TOutputProtocol, TStructIdentifier, TType,
};

use crate::encoding::decode_lower_hex;
use crate::event::NostrEvent;

fn list_len(length: usize, field: &'static str) -> Result<i32, ThriftCompactError> {
    i32::try_from(length).map_err(|_| ThriftCompactError::TooManyItems(field))
}

fn write_tag_value(
    output: &mut dyn TOutputProtocol,
    value: &str,
) -> Result<(), ThriftCompactError> {
    output.write_struct_begin(&TStructIdentifier::new("TagValue"))?;
    if let Some(bytes) = decode_lower_hex(value) {
        output.write_field_begin(&TFieldIdentifier::new("hex", TType::String, 2))?;
        output.write_bytes(&bytes)?;
    } else {
        output.write_field_begin(&TFieldIdentifier::new("text", TType::String, 1))?;
        output.write_string(value)?;
    }
    output.write_field_end()?;
    output.write_field_stop()?;
    output.write_struct_end()?;
    Ok(())
}

fn write_tag(output: &mut dyn TOutputProtocol, tag: &[String]) -> Result<(), ThriftCompactError> {
    output.write_struct_begin(&TStructIdentifier::new("Tag"))?;
    output.write_field_begin(&TFieldIdentifier::new("values", TType::List, 1))?;
    output.write_list_begin(&TListIdentifier::new(
        TType::Struct,
        list_len(tag.len(), "tag values")?,
    ))?;
    for value in tag {
        write_tag_value(output, value)?;
    }
    output.write_list_end()?;
    output.write_field_end()?;
    output.write_field_stop()?;
    output.write_struct_end()?;
    Ok(())
}

fn write_event(
    output: &mut dyn TOutputProtocol,
    event: &NostrEvent,
) -> Result<(), ThriftCompactError> {
    output.write_struct_begin(&TStructIdentifier::new("NostrEvent"))?;

    output.write_field_begin(&TFieldIdentifier::new("id", TType::String, 1))?;
    output.write_bytes(&event.id)?;
    output.write_field_end()?;

    output.write_field_begin(&TFieldIdentifier::new("pubkey", TType::String, 2))?;
    output.write_bytes(&event.pubkey)?;
    output.write_field_end()?;

    output.write_field_begin(&TFieldIdentifier::new("created_at", TType::I64, 3))?;
    output.write_i64(event.created_at)?;
    output.write_field_end()?;

    output.write_field_begin(&TFieldIdentifier::new("kind", TType::I16, 4))?;
    output.write_i16(event.kind as i16)?;
    output.write_field_end()?;

    output.write_field_begin(&TFieldIdentifier::new("tags", TType::List, 5))?;
    output.write_list_begin(&TListIdentifier::new(
        TType::Struct,
        list_len(event.tags.len(), "tags")?,
    ))?;
    for tag in &event.tags {
        write_tag(output, tag)?;
    }
    output.write_list_end()?;
    output.write_field_end()?;

    output.write_field_begin(&TFieldIdentifier::new("content", TType::String, 6))?;
    output.write_string(&event.content)?;
    output.write_field_end()?;

    output.write_field_begin(&TFieldIdentifier::new("sig", TType::String, 7))?;
    output.write_bytes(&event.sig)?;
    output.write_field_end()?;

    output.write_field_stop()?;
    output.write_struct_end()?;
    Ok(())
}

fn expect_type(
    actual: TType,
    expected: TType,
    field: &'static str,
) -> Result<(), ThriftCompactError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ThriftCompactError::InvalidType {
            field,
            expected,
            actual,
        })
    }
}

fn read_tag_value(input: &mut dyn TInputProtocol) -> Result<String, ThriftCompactError> {
    input.read_struct_begin()?;
    let mut value = None;
    loop {
        let field = input.read_field_begin()?;
        if field.field_type == TType::Stop {
            break;
        }
        match field.id {
            Some(1) => {
                expect_type(field.field_type, TType::String, "TagValue.text")?;
                if value.replace(input.read_string()?).is_some() {
                    return Err(ThriftCompactError::DuplicateField("TagValue"));
                }
            }
            Some(2) => {
                expect_type(field.field_type, TType::String, "TagValue.hex")?;
                if value.replace(hex::encode(input.read_bytes()?)).is_some() {
                    return Err(ThriftCompactError::DuplicateField("TagValue"));
                }
            }
            _ => input.skip(field.field_type)?,
        }
        input.read_field_end()?;
    }
    input.read_struct_end()?;
    value.ok_or(ThriftCompactError::MissingField("TagValue.text|hex"))
}

fn read_tag(input: &mut dyn TInputProtocol) -> Result<Vec<String>, ThriftCompactError> {
    input.read_struct_begin()?;
    let mut values = None;
    loop {
        let field = input.read_field_begin()?;
        if field.field_type == TType::Stop {
            break;
        }
        match field.id {
            Some(1) => {
                expect_type(field.field_type, TType::List, "Tag.values")?;
                let list = input.read_list_begin()?;
                expect_type(list.element_type, TType::Struct, "Tag.values[]")?;
                if list.size < 0 {
                    return Err(ThriftCompactError::NegativeList("Tag.values"));
                }
                let mut decoded = Vec::with_capacity(list.size as usize);
                for _ in 0..list.size {
                    decoded.push(read_tag_value(input)?);
                }
                input.read_list_end()?;
                if values.replace(decoded).is_some() {
                    return Err(ThriftCompactError::DuplicateField("Tag.values"));
                }
            }
            _ => input.skip(field.field_type)?,
        }
        input.read_field_end()?;
    }
    input.read_struct_end()?;
    values.ok_or(ThriftCompactError::MissingField("Tag.values"))
}

fn fixed<const N: usize>(
    bytes: Vec<u8>,
    field: &'static str,
) -> Result<[u8; N], ThriftCompactError> {
    bytes
        .try_into()
        .map_err(|_| ThriftCompactError::InvalidLength(field))
}

fn read_event(input: &mut dyn TInputProtocol) -> Result<NostrEvent, ThriftCompactError> {
    input.read_struct_begin()?;
    let (mut id, mut pubkey, mut created_at, mut kind) = (None, None, None, None);
    let (mut tags, mut content, mut sig) = (None, None, None);

    loop {
        let field = input.read_field_begin()?;
        if field.field_type == TType::Stop {
            break;
        }
        match field.id {
            Some(1) => {
                expect_type(field.field_type, TType::String, "id")?;
                id = Some(fixed(input.read_bytes()?, "id")?);
            }
            Some(2) => {
                expect_type(field.field_type, TType::String, "pubkey")?;
                pubkey = Some(fixed(input.read_bytes()?, "pubkey")?);
            }
            Some(3) => {
                expect_type(field.field_type, TType::I64, "created_at")?;
                created_at = Some(input.read_i64()?);
            }
            Some(4) => {
                expect_type(field.field_type, TType::I16, "kind")?;
                kind = Some(input.read_i16()? as u16);
            }
            Some(5) => {
                expect_type(field.field_type, TType::List, "tags")?;
                let list = input.read_list_begin()?;
                expect_type(list.element_type, TType::Struct, "tags[]")?;
                if list.size < 0 {
                    return Err(ThriftCompactError::NegativeList("tags"));
                }
                let mut decoded = Vec::with_capacity(list.size as usize);
                for _ in 0..list.size {
                    decoded.push(read_tag(input)?);
                }
                input.read_list_end()?;
                tags = Some(decoded);
            }
            Some(6) => {
                expect_type(field.field_type, TType::String, "content")?;
                content = Some(input.read_string()?);
            }
            Some(7) => {
                expect_type(field.field_type, TType::String, "sig")?;
                sig = Some(fixed(input.read_bytes()?, "sig")?);
            }
            _ => input.skip(field.field_type)?,
        }
        input.read_field_end()?;
    }
    input.read_struct_end()?;

    Ok(NostrEvent {
        id: id.ok_or(ThriftCompactError::MissingField("id"))?,
        pubkey: pubkey.ok_or(ThriftCompactError::MissingField("pubkey"))?,
        created_at: created_at.ok_or(ThriftCompactError::MissingField("created_at"))?,
        kind: kind.ok_or(ThriftCompactError::MissingField("kind"))?,
        tags: tags.ok_or(ThriftCompactError::MissingField("tags"))?,
        content: content.ok_or(ThriftCompactError::MissingField("content"))?,
        sig: sig.ok_or(ThriftCompactError::MissingField("sig"))?,
    })
}

pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut output = TCompactOutputProtocol::new(&mut bytes);
        write_event(&mut output, event).expect("Thrift serialization should not fail");
        output.flush().expect("Thrift flush should not fail");
    }
    bytes
}

pub fn deserialize(data: &[u8]) -> Result<NostrEvent, ThriftCompactError> {
    let mut cursor = Cursor::new(data);
    let event = {
        let mut input = TCompactInputProtocol::new(&mut cursor);
        read_event(&mut input)?
    };
    if cursor.position() != data.len() as u64 {
        return Err(ThriftCompactError::TrailingBytes(
            data.len() - cursor.position() as usize,
        ));
    }
    Ok(event)
}

pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut output = TCompactOutputProtocol::new(&mut bytes);
        output
            .write_struct_begin(&TStructIdentifier::new("EventBatch"))
            .expect("Thrift serialization should not fail");
        output
            .write_field_begin(&TFieldIdentifier::new("events", TType::List, 1))
            .expect("Thrift serialization should not fail");
        output
            .write_list_begin(&TListIdentifier::new(
                TType::Struct,
                list_len(events.len(), "events").expect("batch fits in Thrift list"),
            ))
            .expect("Thrift serialization should not fail");
        for event in events {
            write_event(&mut output, event).expect("Thrift serialization should not fail");
        }
        output
            .write_list_end()
            .expect("Thrift serialization should not fail");
        output
            .write_field_end()
            .expect("Thrift serialization should not fail");
        output
            .write_field_stop()
            .expect("Thrift serialization should not fail");
        output
            .write_struct_end()
            .expect("Thrift serialization should not fail");
        output.flush().expect("Thrift flush should not fail");
    }
    bytes
}

pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, ThriftCompactError> {
    let mut cursor = Cursor::new(data);
    let events = {
        let mut input = TCompactInputProtocol::new(&mut cursor);
        input.read_struct_begin()?;
        let mut events = None;
        loop {
            let field = input.read_field_begin()?;
            if field.field_type == TType::Stop {
                break;
            }
            match field.id {
                Some(1) => {
                    expect_type(field.field_type, TType::List, "events")?;
                    let list = input.read_list_begin()?;
                    expect_type(list.element_type, TType::Struct, "events[]")?;
                    if list.size < 0 {
                        return Err(ThriftCompactError::NegativeList("events"));
                    }
                    let mut decoded = Vec::with_capacity(list.size as usize);
                    for _ in 0..list.size {
                        decoded.push(read_event(&mut input)?);
                    }
                    input.read_list_end()?;
                    events = Some(decoded);
                }
                _ => input.skip(field.field_type)?,
            }
            input.read_field_end()?;
        }
        input.read_struct_end()?;
        events.ok_or(ThriftCompactError::MissingField("events"))?
    };
    if cursor.position() != data.len() as u64 {
        return Err(ThriftCompactError::TrailingBytes(
            data.len() - cursor.position() as usize,
        ));
    }
    Ok(events)
}

#[derive(Debug, thiserror::Error)]
pub enum ThriftCompactError {
    #[error("Thrift error: {0}")]
    Thrift(#[from] thrift::Error),

    #[error("missing Thrift field: {0}")]
    MissingField(&'static str),

    #[error("duplicate Thrift field: {0}")]
    DuplicateField(&'static str),

    #[error("invalid {0} length")]
    InvalidLength(&'static str),

    #[error("too many items for Thrift list: {0}")]
    TooManyItems(&'static str),

    #[error("negative Thrift list size for {0}")]
    NegativeList(&'static str),

    #[error("invalid Thrift type for {field}: expected {expected}, got {actual}")]
    InvalidType {
        field: &'static str,
        expected: TType,
        actual: TType,
    },

    #[error("Thrift input has {0} trailing bytes")]
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
            kind: 60_000,
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
