//! bincode 2 Rust/Serde ecosystem reference encoding.
//!
//! This is not a cross-language protocol candidate. The profile uses bincode
//! 2's standard variable-integer configuration over `reference_wire`.

use crate::event::NostrEvent;
use crate::reference_wire::{EventOwned, EventRef, EventsRef};

fn config() -> impl ::bincode::config::Config {
    ::bincode::config::standard()
}

pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    ::bincode::serde::encode_to_vec(EventRef::from(event), config())
        .expect("bincode serialization should not fail")
}

pub fn deserialize(data: &[u8]) -> Result<NostrEvent, BincodeError> {
    let (event, consumed): (EventOwned, usize) =
        ::bincode::serde::decode_from_slice(data, config())?;
    if consumed != data.len() {
        return Err(BincodeError::TrailingBytes(data.len() - consumed));
    }
    Ok(event.into())
}

pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    ::bincode::serde::encode_to_vec(EventsRef(events), config())
        .expect("bincode serialization should not fail")
}

pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, BincodeError> {
    let (events, consumed): (Vec<EventOwned>, usize) =
        ::bincode::serde::decode_from_slice(data, config())?;
    if consumed != data.len() {
        return Err(BincodeError::TrailingBytes(data.len() - consumed));
    }
    Ok(events.into_iter().map(NostrEvent::from).collect())
}

#[derive(Debug, thiserror::Error)]
pub enum BincodeError {
    #[error("bincode decode error: {0}")]
    Decode(#[from] ::bincode::error::DecodeError),

    #[error("bincode input has {0} trailing bytes")]
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
