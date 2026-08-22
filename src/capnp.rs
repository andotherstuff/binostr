//! Cap'n Proto serialization - MAXIMUM COMPRESSION
//!
//! All fixed-size fields packed into single 138-byte blob:
//!   - id: 32 bytes (offset 0)
//!   - pubkey: 32 bytes (offset 32)
//!   - sig: 64 bytes (offset 64)
//!   - createdAt: 8 bytes i64 LE (offset 128)
//!   - kind: 2 bytes u16 LE (offset 136)
//!
//! Tags packed into single blob with length-prefixed values.
//! Only 3 Cap'n Proto pointers: fixedData, tagData, content.

use capnp::message::{Builder, ReaderOptions};
use capnp::serialize;
use capnp::serialize_packed;

use crate::encoding::decode_lower_hex;
use crate::event::NostrEvent;

// Include the generated Cap'n Proto code
pub mod nostr_capnp {
    include!(concat!(env!("OUT_DIR"), "/nostr_capnp.rs"));
}

use nostr_capnp::nostr_event;

/// Fixed data size: id(32) + pubkey(32) + sig(64) + created_at(8) + kind(2) = 138 bytes
const FIXED_DATA_SIZE: usize = 138;

/// Pack all fixed fields into a 138-byte blob
#[inline]
fn pack_fixed_data(event: &NostrEvent) -> [u8; FIXED_DATA_SIZE] {
    let mut buf = [0u8; FIXED_DATA_SIZE];
    buf[0..32].copy_from_slice(&event.id);
    buf[32..64].copy_from_slice(&event.pubkey);
    buf[64..128].copy_from_slice(&event.sig);
    buf[128..136].copy_from_slice(&event.created_at.to_le_bytes());
    buf[136..138].copy_from_slice(&event.kind.to_le_bytes());
    buf
}

/// Unpack fixed fields from a 138-byte blob
#[inline]
#[allow(clippy::type_complexity)]
fn unpack_fixed_data(data: &[u8]) -> Result<([u8; 32], [u8; 32], [u8; 64], i64, u16), CapnpError> {
    if data.len() < FIXED_DATA_SIZE {
        return Err(CapnpError::InvalidLength("fixed data too short"));
    }

    let id: [u8; 32] = data[0..32].try_into().unwrap();
    let pubkey: [u8; 32] = data[32..64].try_into().unwrap();
    let sig: [u8; 64] = data[64..128].try_into().unwrap();
    let created_at = i64::from_le_bytes(data[128..136].try_into().unwrap());
    let kind = u16::from_le_bytes(data[136..138].try_into().unwrap());

    Ok((id, pubkey, sig, created_at, kind))
}

/// Encode a tag value optimally: if it's hex, decode to bytes (50% size reduction),
/// otherwise store as UTF-8 bytes
fn encode_tag_value(value: &str) -> (bool, Vec<u8>) {
    if let Some(bytes) = decode_lower_hex(value) {
        return (true, bytes);
    }
    // Store as raw UTF-8 bytes
    (false, value.as_bytes().to_vec())
}

/// Decode a tag value from bytes back to string
fn decode_tag_value(is_hex: bool, bytes: &[u8]) -> Result<String, CapnpError> {
    if is_hex {
        // Encode hex bytes back to hex string
        Ok(hex::encode(bytes))
    } else {
        // Decode UTF-8 bytes back to string
        Ok(std::str::from_utf8(bytes)?.to_string())
    }
}

/// Pack all tags into a single compact blob
/// Format: [tag_count:u16] then for each tag: [value_count:u8] then for each value:
///         [flags_and_len:u16 where bit15=is_hex, bits0-14=length][data]
fn pack_tags(tags: &[Vec<String>]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);

    // Tag count (u16)
    buf.extend_from_slice(&(tags.len() as u16).to_le_bytes());

    for tag in tags {
        // Value count for this tag (u8)
        buf.push(tag.len() as u8);

        for value in tag {
            let (is_hex, data) = encode_tag_value(value);
            // flags_and_len: bit15 = is_hex, bits0-14 = length
            let flags_and_len = if is_hex {
                0x8000 | (data.len() as u16)
            } else {
                data.len() as u16
            };
            buf.extend_from_slice(&flags_and_len.to_le_bytes());
            buf.extend_from_slice(&data);
        }
    }

    buf
}

/// Unpack tags from a compact blob
fn unpack_tags(data: &[u8]) -> Result<Vec<Vec<String>>, CapnpError> {
    if data.len() < 2 {
        return Ok(Vec::new());
    }

    let mut pos = 0;

    // Read tag count
    let tag_count = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;

    let mut tags = Vec::with_capacity(tag_count);

    for _ in 0..tag_count {
        if pos >= data.len() {
            return Err(CapnpError::InvalidTagData("truncated tag data"));
        }

        // Read value count for this tag
        let value_count = data[pos] as usize;
        pos += 1;

        let mut values = Vec::with_capacity(value_count);

        for _ in 0..value_count {
            if pos + 2 > data.len() {
                return Err(CapnpError::InvalidTagData("truncated value header"));
            }

            // Read flags_and_len
            let flags_and_len = u16::from_le_bytes([data[pos], data[pos + 1]]);
            pos += 2;

            let is_hex = (flags_and_len & 0x8000) != 0;
            let len = (flags_and_len & 0x7FFF) as usize;

            if pos + len > data.len() {
                return Err(CapnpError::InvalidTagData("truncated value data"));
            }

            let value_bytes = &data[pos..pos + len];
            pos += len;

            values.push(decode_tag_value(is_hex, value_bytes)?);
        }

        tags.push(values);
    }

    Ok(tags)
}

/// Serialize a NostrEvent to Cap'n Proto format
pub fn serialize_event(event: &NostrEvent) -> Vec<u8> {
    let mut message = Builder::new_default();

    {
        let mut builder = message.init_root::<nostr_event::Builder>();

        // Pack all fixed fields into single 138-byte blob
        let fixed_data = pack_fixed_data(event);
        builder.set_fixed_data(&fixed_data);

        // Pack all tags into single blob
        let tag_data = pack_tags(&event.tags);
        builder.set_tag_data(&tag_data);

        builder.set_content(&event.content);
    }

    let mut buf = Vec::new();
    serialize::write_message(&mut buf, &message).expect("Cap'n Proto serialization failed");
    buf
}

/// Deserialize a NostrEvent from Cap'n Proto format
pub fn deserialize_event(data: &[u8]) -> Result<NostrEvent, CapnpError> {
    let reader = serialize::read_message(data, ReaderOptions::new())?;
    let event_reader = reader.get_root::<nostr_event::Reader>()?;

    // Unpack all fixed fields from single blob
    let fixed_data = event_reader.get_fixed_data()?;
    let (id, pubkey, sig, created_at, kind) = unpack_fixed_data(fixed_data)?;

    // Unpack tags from single blob
    let tag_data = event_reader.get_tag_data()?;
    let tags = unpack_tags(tag_data)?;

    let content = event_reader.get_content()?.to_string()?;

    Ok(NostrEvent {
        id,
        pubkey,
        created_at,
        kind,
        tags,
        content,
        sig,
    })
}

/// Serialize a NostrEvent to Cap'n Proto packed format (compressed)
pub fn serialize_event_packed(event: &NostrEvent) -> Vec<u8> {
    let mut message = Builder::new_default();

    {
        let mut builder = message.init_root::<nostr_event::Builder>();

        // Pack all fixed fields into single 138-byte blob
        let fixed_data = pack_fixed_data(event);
        builder.set_fixed_data(&fixed_data);

        // Pack all tags into single blob
        let tag_data = pack_tags(&event.tags);
        builder.set_tag_data(&tag_data);

        builder.set_content(&event.content);
    }

    let mut buf = Vec::new();
    serialize_packed::write_message(&mut buf, &message)
        .expect("Cap'n Proto packed serialization failed");
    buf
}

/// Deserialize a NostrEvent from Cap'n Proto packed format
pub fn deserialize_event_packed(data: &[u8]) -> Result<NostrEvent, CapnpError> {
    let reader = serialize_packed::read_message(data, ReaderOptions::new())?;
    let event_reader = reader.get_root::<nostr_event::Reader>()?;

    // Unpack all fixed fields from single blob
    let fixed_data = event_reader.get_fixed_data()?;
    let (id, pubkey, sig, created_at, kind) = unpack_fixed_data(fixed_data)?;

    // Unpack tags from single blob
    let tag_data = event_reader.get_tag_data()?;
    let tags = unpack_tags(tag_data)?;

    let content = event_reader.get_content()?.to_string()?;

    Ok(NostrEvent {
        id,
        pubkey,
        created_at,
        kind,
        tags,
        content,
        sig,
    })
}

/// Serialize a batch of events to Cap'n Proto format
pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    use nostr_capnp::event_batch;

    let mut message = Builder::new_default();

    {
        let builder = message.init_root::<event_batch::Builder>();
        let mut events_builder = builder.init_events(events.len() as u32);

        for (i, event) in events.iter().enumerate() {
            let mut event_builder = events_builder.reborrow().get(i as u32);

            // Pack all fixed fields into single 138-byte blob
            let fixed_data = pack_fixed_data(event);
            event_builder.set_fixed_data(&fixed_data);

            // Pack all tags into single blob
            let tag_data = pack_tags(&event.tags);
            event_builder.set_tag_data(&tag_data);

            event_builder.set_content(&event.content);
        }
    }

    let mut buf = Vec::new();
    serialize::write_message(&mut buf, &message).expect("Cap'n Proto serialization failed");
    buf
}

/// Deserialize a batch of events from Cap'n Proto format
pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, CapnpError> {
    use nostr_capnp::event_batch;

    let reader = serialize::read_message(data, ReaderOptions::new())?;
    let batch_reader = reader.get_root::<event_batch::Reader>()?;
    let events_reader = batch_reader.get_events()?;

    let mut events = Vec::with_capacity(events_reader.len() as usize);

    for event_reader in events_reader.iter() {
        // Unpack all fixed fields from single blob
        let fixed_data = event_reader.get_fixed_data()?;
        let (id, pubkey, sig, created_at, kind) = unpack_fixed_data(fixed_data)?;

        // Unpack tags from single blob
        let tag_data = event_reader.get_tag_data()?;
        let tags = unpack_tags(tag_data)?;

        let content = event_reader.get_content()?.to_string()?;

        events.push(NostrEvent {
            id,
            pubkey,
            created_at,
            kind,
            tags,
            content,
            sig,
        });
    }

    Ok(events)
}

/// Serialize a batch of events to Cap'n Proto packed format (compressed)
pub fn serialize_batch_packed(events: &[NostrEvent]) -> Vec<u8> {
    use nostr_capnp::event_batch;

    let mut message = Builder::new_default();

    {
        let builder = message.init_root::<event_batch::Builder>();
        let mut events_builder = builder.init_events(events.len() as u32);

        for (i, event) in events.iter().enumerate() {
            let mut event_builder = events_builder.reborrow().get(i as u32);

            // Pack all fixed fields into single 138-byte blob
            let fixed_data = pack_fixed_data(event);
            event_builder.set_fixed_data(&fixed_data);

            // Pack all tags into single blob
            let tag_data = pack_tags(&event.tags);
            event_builder.set_tag_data(&tag_data);

            event_builder.set_content(&event.content);
        }
    }

    let mut buf = Vec::new();
    serialize_packed::write_message(&mut buf, &message)
        .expect("Cap'n Proto packed serialization failed");
    buf
}

/// Deserialize a batch of events from Cap'n Proto packed format
pub fn deserialize_batch_packed(data: &[u8]) -> Result<Vec<NostrEvent>, CapnpError> {
    use nostr_capnp::event_batch;

    let reader = serialize_packed::read_message(data, ReaderOptions::new())?;
    let batch_reader = reader.get_root::<event_batch::Reader>()?;
    let events_reader = batch_reader.get_events()?;

    let mut events = Vec::with_capacity(events_reader.len() as usize);

    for event_reader in events_reader.iter() {
        // Unpack all fixed fields from single blob
        let fixed_data = event_reader.get_fixed_data()?;
        let (id, pubkey, sig, created_at, kind) = unpack_fixed_data(fixed_data)?;

        // Unpack tags from single blob
        let tag_data = event_reader.get_tag_data()?;
        let tags = unpack_tags(tag_data)?;

        let content = event_reader.get_content()?.to_string()?;

        events.push(NostrEvent {
            id,
            pubkey,
            created_at,
            kind,
            tags,
            content,
            sig,
        });
    }

    Ok(events)
}

// ============================================
// Zero-copy field access (Cap'n Proto's strength)
// ============================================

/// Read only the `kind` field from a Cap'n Proto encoded event without full deserialization.
/// This demonstrates Cap'n Proto's zero-copy advantage for selective field access.
pub fn read_kind(data: &[u8]) -> Result<u16, CapnpError> {
    let reader = serialize::read_message(data, ReaderOptions::new())?;
    let event_reader = reader.get_root::<nostr_event::Reader>()?;

    let fixed_data = event_reader.get_fixed_data()?;
    if fixed_data.len() < FIXED_DATA_SIZE {
        return Err(CapnpError::InvalidLength("fixed data too short"));
    }

    Ok(u16::from_le_bytes([fixed_data[136], fixed_data[137]]))
}

/// Read only the `created_at` field from a Cap'n Proto encoded event without full deserialization.
pub fn read_created_at(data: &[u8]) -> Result<i64, CapnpError> {
    let reader = serialize::read_message(data, ReaderOptions::new())?;
    let event_reader = reader.get_root::<nostr_event::Reader>()?;

    let fixed_data = event_reader.get_fixed_data()?;
    if fixed_data.len() < FIXED_DATA_SIZE {
        return Err(CapnpError::InvalidLength("fixed data too short"));
    }

    Ok(i64::from_le_bytes(fixed_data[128..136].try_into().unwrap()))
}

/// Read only the `id` field from a Cap'n Proto encoded event without full deserialization.
pub fn read_id(data: &[u8]) -> Result<[u8; 32], CapnpError> {
    let reader = serialize::read_message(data, ReaderOptions::new())?;
    let event_reader = reader.get_root::<nostr_event::Reader>()?;

    let fixed_data = event_reader.get_fixed_data()?;
    if fixed_data.len() < FIXED_DATA_SIZE {
        return Err(CapnpError::InvalidLength("fixed data too short"));
    }

    Ok(fixed_data[0..32].try_into().unwrap())
}

/// Read only the `pubkey` field from a Cap'n Proto encoded event without full deserialization.
pub fn read_pubkey(data: &[u8]) -> Result<[u8; 32], CapnpError> {
    let reader = serialize::read_message(data, ReaderOptions::new())?;
    let event_reader = reader.get_root::<nostr_event::Reader>()?;

    let fixed_data = event_reader.get_fixed_data()?;
    if fixed_data.len() < FIXED_DATA_SIZE {
        return Err(CapnpError::InvalidLength("fixed data too short"));
    }

    Ok(fixed_data[32..64].try_into().unwrap())
}

/// Read multiple fields (kind + pubkey) from a Cap'n Proto encoded event.
/// This shows the efficiency of reading multiple fields with a single message parse.
pub fn read_kind_and_pubkey(data: &[u8]) -> Result<(u16, [u8; 32]), CapnpError> {
    let reader = serialize::read_message(data, ReaderOptions::new())?;
    let event_reader = reader.get_root::<nostr_event::Reader>()?;

    let fixed_data = event_reader.get_fixed_data()?;
    if fixed_data.len() < FIXED_DATA_SIZE {
        return Err(CapnpError::InvalidLength("fixed data too short"));
    }

    let kind = u16::from_le_bytes([fixed_data[136], fixed_data[137]]);
    let pubkey: [u8; 32] = fixed_data[32..64].try_into().unwrap();

    Ok((kind, pubkey))
}

#[derive(Debug, thiserror::Error)]
pub enum CapnpError {
    #[error("Cap'n Proto error: {0}")]
    Capnp(#[from] capnp::Error),

    #[error("Cap'n Proto not in schema: {0}")]
    NotInSchema(#[from] capnp::NotInSchema),

    #[error("Invalid length for field: {0}")]
    InvalidLength(&'static str),

    #[error("Invalid tag data: {0}")]
    InvalidTagData(&'static str),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
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
    fn test_roundtrip() {
        let event = sample_event();
        let bytes = serialize_event(&event);
        let back = deserialize_event(&bytes).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_packed_roundtrip() {
        let event = sample_event();
        let bytes = serialize_event_packed(&event);
        let back = deserialize_event_packed(&bytes).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_batch_roundtrip() {
        let events = vec![sample_event(), sample_event()];
        let bytes = serialize_batch(&events);
        let back = deserialize_batch(&bytes).unwrap();
        assert_eq!(events, back);
    }

    #[test]
    fn test_batch_packed_roundtrip() {
        let events = vec![sample_event(), sample_event()];
        let bytes = serialize_batch_packed(&events);
        let back = deserialize_batch_packed(&bytes).unwrap();
        assert_eq!(events, back);
    }

    #[test]
    fn test_size_comparison() {
        let event = sample_event();

        let capnp_size = serialize_event(&event).len();
        let capnp_packed_size = serialize_event_packed(&event).len();
        let json_size = crate::json::serialize(&event).len();

        println!("Cap'n Proto:        {} bytes", capnp_size);
        println!("Cap'n Proto Packed: {} bytes", capnp_packed_size);
        println!("JSON:               {} bytes", json_size);
        println!(
            "Packed savings:     {:.1}%",
            100.0 * (1.0 - capnp_packed_size as f64 / capnp_size as f64)
        );

        // Packed should be smaller than unpacked
        assert!(capnp_packed_size <= capnp_size);
    }
}
