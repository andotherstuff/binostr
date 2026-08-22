//! Standard MessagePack encoding for Nostr events.
//!
//! Events use the same positional standard-data-model profile as packed CBOR.
//! Fixed cryptographic fields use MessagePack binary values. Lowercase
//! hexadecimal tag strings may also use binary values; the MessagePack type
//! marker keeps that transformation reversible.

use crate::event::NostrEvent;
use crate::positional::{EventOwned, EventRef, EventsRef, PositionalError};

pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    rmp_serde::to_vec(&EventRef::from(event)).expect("MessagePack serialization should not fail")
}

pub fn deserialize(data: &[u8]) -> Result<NostrEvent, MessagePackError> {
    let event: EventOwned = rmp_serde::from_slice(data)?;
    event.try_into().map_err(profile_error)
}

pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    rmp_serde::to_vec(&EventsRef(events)).expect("MessagePack serialization should not fail")
}

pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, MessagePackError> {
    let events: Vec<EventOwned> = rmp_serde::from_slice(data)?;
    events
        .into_iter()
        .map(|event| event.try_into().map_err(profile_error))
        .collect()
}

fn profile_error(error: PositionalError) -> MessagePackError {
    match error {
        PositionalError::InvalidLength(field) => MessagePackError::InvalidLength(field),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MessagePackError {
    #[error("MessagePack decode error: {0}")]
    Decode(#[from] rmp_serde::decode::Error),

    #[error("invalid {0} length")]
    InvalidLength(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> NostrEvent {
        NostrEvent {
            id: [0xab; 32],
            pubkey: [0xcd; 32],
            created_at: 1_700_000_000,
            kind: 1,
            tags: vec![
                vec!["e".into(), hex::encode([0xef; 32])],
                vec!["custom".into(), "DEADBEEF".into()],
            ],
            content: "Hello, Nostr!".into(),
            sig: [0x12; 64],
        }
    }

    #[test]
    fn roundtrip_preserves_arbitrary_tag_text() {
        let event = sample_event();
        assert_eq!(deserialize(&serialize(&event)).unwrap(), event);
    }

    #[test]
    fn batch_roundtrip() {
        let events = vec![sample_event(), sample_event()];
        assert_eq!(
            deserialize_batch(&serialize_batch(&events)).unwrap(),
            events
        );
    }
}
