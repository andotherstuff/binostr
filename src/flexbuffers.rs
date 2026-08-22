//! FlexBuffers encoding using the shared positional Nostr profile.
//!
//! FlexBuffers is the official schema-less companion to FlatBuffers. Using the
//! same typed model as packed CBOR, MessagePack, and BEVE keeps the owned-codec
//! comparison focused on the format implementation.

use crate::event::NostrEvent;
use crate::positional::{EventOwned, EventRef, EventsRef, PositionalError};

pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    flexbuffers::to_vec(EventRef::from(event)).expect("FlexBuffers serialization should not fail")
}

pub fn deserialize(data: &[u8]) -> Result<NostrEvent, FlexBuffersError> {
    let event: EventOwned = flexbuffers::from_slice(data)?;
    event.try_into().map_err(profile_error)
}

pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    flexbuffers::to_vec(EventsRef(events)).expect("FlexBuffers serialization should not fail")
}

pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, FlexBuffersError> {
    let events: Vec<EventOwned> = flexbuffers::from_slice(data)?;
    events
        .into_iter()
        .map(|event| event.try_into().map_err(profile_error))
        .collect()
}

/// Read relay-filter fields without materializing the complete event.
pub fn read_kind_and_pubkey(data: &[u8]) -> Result<(u16, &[u8; 32]), FlexBuffersError> {
    let root = flexbuffers::Reader::get_root(data)?;
    let event = root.get_vector()?;
    if event.len() != 7 {
        return Err(FlexBuffersError::InvalidLength("event vector"));
    }
    let kind = u16::try_from(event.index(3)?.get_u64()?)
        .map_err(|_| FlexBuffersError::InvalidLength("kind"))?;
    let pubkey = event
        .index(1)?
        .get_blob()?
        .0
        .try_into()
        .map_err(|_| FlexBuffersError::InvalidLength("pubkey"))?;
    Ok((kind, pubkey))
}

fn profile_error(error: PositionalError) -> FlexBuffersError {
    match error {
        PositionalError::InvalidLength(field) => FlexBuffersError::InvalidLength(field),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FlexBuffersError {
    #[error("FlexBuffers decode error: {0}")]
    Decode(#[from] flexbuffers::DeserializationError),

    #[error("FlexBuffers reader error: {0}")]
    Reader(#[from] flexbuffers::ReaderError),

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
            kind: 30_023,
            tags: vec![
                vec!["e".into(), hex::encode([0xef; 32])],
                vec!["custom".into(), "DEADBEEF".into()],
            ],
            content: "Hello, Nostr!".into(),
            sig: [0x12; 64],
        }
    }

    #[test]
    fn roundtrip() {
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

    #[test]
    fn selective_access() {
        let event = sample_event();
        let bytes = serialize(&event);
        let (kind, pubkey) = read_kind_and_pubkey(&bytes).unwrap();
        assert_eq!(kind, event.kind);
        assert_eq!(pubkey, &event.pubkey);
    }
}
