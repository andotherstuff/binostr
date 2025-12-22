//! Notepack: Compact binary format for Nostr events
//!
//! Notepack is a compact binary format designed specifically for Nostr events,
//! using varint encoding and optimized string storage.
//!
//! See: <https://docs.rs/notepack>

use crate::event::NostrEvent;
use notepack::{NoteBinary, NoteParser, StringType};

/// Error type for notepack serialization/deserialization
#[derive(Debug, thiserror::Error)]
pub enum NotepackError {
    #[error("Pack error: {0}")]
    Pack(#[from] notepack::Error),

    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("Missing field: {0}")]
    MissingField(&'static str),

    #[error("Invalid field size: expected {expected}, got {actual}")]
    InvalidFieldSize { expected: usize, actual: usize },
}

/// Serialize a NostrEvent to notepack binary format
///
/// Uses `NoteBinary` for zero-allocation serialization - directly passes
/// binary fields (id, pubkey, sig) without hex encoding/decoding overhead.
pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    let note = NoteBinary {
        id: &event.id,
        pubkey: &event.pubkey,
        sig: &event.sig,
        created_at: event.created_at as u64,
        kind: event.kind as u64,
        tags: &event.tags,
        content: &event.content,
    };
    note.pack()
}

/// Deserialize a NostrEvent from notepack binary format
pub fn deserialize(data: &[u8]) -> Result<NostrEvent, NotepackError> {
    let parser = NoteParser::new(data);
    let note = parser.into_note()?;

    // Copy fixed-size arrays (note.id/pubkey/sig are already &[u8; N])
    let id: [u8; 32] = *note.id;
    let pubkey: [u8; 32] = *note.pubkey;
    let sig: [u8; 64] = *note.sig;

    // Parse tags from the lazy iterator
    let mut tags_vec = Vec::new();
    let mut tags = note.tags;
    while let Some(elems) = tags.next_tag()? {
        let mut tag_values = Vec::new();
        for elem in elems {
            match elem? {
                StringType::Str(s) => tag_values.push(s.to_string()),
                StringType::Bytes(bs) => tag_values.push(hex::encode(bs)),
            }
        }
        tags_vec.push(tag_values);
    }

    Ok(NostrEvent {
        id,
        pubkey,
        created_at: note.created_at as i64,
        kind: note.kind as u16,
        tags: tags_vec,
        content: note.content.to_string(),
        sig,
    })
}

/// Serialize a batch of events to notepack format
///
/// Format: [count: u32 LE][len1: u32 LE][data1][len2: u32 LE][data2]...
pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    let serialized: Vec<Vec<u8>> = events.iter().map(serialize).collect();
    let total_size: usize = 4 + serialized.iter().map(|e| 4 + e.len()).sum::<usize>();

    let mut buf = Vec::with_capacity(total_size);
    buf.extend_from_slice(&(events.len() as u32).to_le_bytes());

    for event_data in &serialized {
        buf.extend_from_slice(&(event_data.len() as u32).to_le_bytes());
        buf.extend_from_slice(event_data);
    }

    buf
}

/// Deserialize a batch of events from notepack format
pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, NotepackError> {
    if data.len() < 4 {
        return Err(NotepackError::MissingField("batch header"));
    }

    let event_count = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    let mut pos = 4;
    let mut events = Vec::with_capacity(event_count);

    for _ in 0..event_count {
        if pos + 4 > data.len() {
            return Err(NotepackError::MissingField("event length"));
        }

        let event_len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;

        if pos + event_len > data.len() {
            return Err(NotepackError::MissingField("event data"));
        }

        events.push(deserialize(&data[pos..pos + event_len])?);
        pos += event_len;
    }

    Ok(events)
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
                vec!["p".to_string(), "abcd1234".to_string()],
                vec!["e".to_string(), "deadbeef".to_string()],
            ],
            content: "Hello, Nostr!".to_string(),
            sig: [0xef; 64],
        }
    }

    #[test]
    fn test_roundtrip() {
        let event = sample_event();
        let bytes = serialize(&event);
        let back = deserialize(&bytes).unwrap();
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
    fn test_empty_content() {
        let event = NostrEvent {
            id: [0x11; 32],
            pubkey: [0x22; 32],
            created_at: 1700000000,
            kind: 1,
            tags: vec![],
            content: String::new(),
            sig: [0x33; 64],
        };

        let bytes = serialize(&event);
        let back = deserialize(&bytes).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_unicode_content() {
        let event = NostrEvent {
            id: [0x44; 32],
            pubkey: [0x55; 32],
            created_at: 1700000001,
            kind: 1,
            tags: vec![],
            content: "Hello 🌍! こんにちは 世界 🚀".to_string(),
            sig: [0x66; 64],
        };

        let bytes = serialize(&event);
        let back = deserialize(&bytes).unwrap();
        assert_eq!(event, back);
    }
}
