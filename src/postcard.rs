//! postcard Rust/Serde ecosystem reference encoding.
//!
//! postcard is included because Rust users commonly ask about it, not as an
//! interoperable Nostr wire-standard candidate.

use crate::event::NostrEvent;
use crate::reference_wire::{EventOwned, EventRef, EventsRef};

pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    ::postcard::to_allocvec(&EventRef::from(event)).expect("postcard serialization should not fail")
}

pub fn deserialize(data: &[u8]) -> Result<NostrEvent, PostcardError> {
    let event: EventOwned = ::postcard::from_bytes(data)?;
    Ok(event.into())
}

pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    ::postcard::to_allocvec(&EventsRef(events)).expect("postcard serialization should not fail")
}

pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, PostcardError> {
    let events: Vec<EventOwned> = ::postcard::from_bytes(data)?;
    Ok(events.into_iter().map(NostrEvent::from).collect())
}

#[derive(Debug, thiserror::Error)]
pub enum PostcardError {
    #[error("postcard decode error: {0}")]
    Decode(#[from] ::postcard::Error),
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
            tags: vec![vec!["e".into(), hex::encode([3; 32])]],
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
