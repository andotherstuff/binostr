//! BEVE encoding as an emerging performance reference.
//!
//! BEVE has a published open specification and a current Rust implementation,
//! but it is not classified as an interoperable SDK candidate here because its
//! target-language coverage is materially narrower than CBOR or MessagePack.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};

use crate::event::NostrEvent;
use crate::positional::{EventRef, EventsRef};

pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    beve::to_vec(&EventRef::from(event)).expect("BEVE serialization should not fail")
}

pub fn deserialize(data: &[u8]) -> Result<NostrEvent, BeveError> {
    beve::validate_slice(data)?;
    let event: BeveEventOwned = beve::from_slice(data)?;
    event.try_into()
}

pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    beve::to_vec(&EventsRef(events)).expect("BEVE serialization should not fail")
}

pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, BeveError> {
    beve::validate_slice(data)?;
    let events: Vec<BeveEventOwned> = beve::from_slice(data)?;
    events.into_iter().map(NostrEvent::try_from).collect()
}

/// Verify that the complete slice contains exactly one structurally valid BEVE value.
pub fn verify(data: &[u8]) -> Result<(), BeveError> {
    beve::validate_slice(data).map_err(BeveError::from)
}

/// Read relay-filter fields while safely skipping untargeted values.
///
/// This checks the navigation paths and selected field types, but does not
/// validate every skipped value. Call [`verify`] once at an ingress boundary
/// before treating repeated selective reads as reads from a validated buffer.
pub fn read_kind_and_pubkey(data: &[u8]) -> Result<(u16, &[u8; 32]), BeveError> {
    let kind: u16 = beve::from_field(data, "/3")?;
    let pubkey: BorrowedBytes<'_> = beve::from_field(data, "/1")?;
    let pubkey = pubkey
        .0
        .try_into()
        .map_err(|_| BeveError::InvalidLength("pubkey"))?;
    Ok((kind, pubkey))
}

struct BorrowedBytes<'a>(&'a [u8]);

impl<'de> Deserialize<'de> for BorrowedBytes<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BytesVisitor;

        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = BorrowedBytes<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a borrowed unsigned-byte typed array")
            }

            fn visit_borrowed_bytes<E>(self, value: &'de [u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BorrowedBytes(value))
            }
        }

        deserializer.deserialize_bytes(BytesVisitor)
    }
}

#[derive(Deserialize)]
struct BeveEventOwned {
    #[serde(with = "serde_bytes")]
    id: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pubkey: Vec<u8>,
    created_at: i64,
    kind: u16,
    tags: Vec<Vec<BeveTagValue>>,
    content: String,
    #[serde(with = "serde_bytes")]
    sig: Vec<u8>,
}

enum BeveTagValue {
    Text(String),
    Hex(Vec<u8>),
}

impl<'de> Deserialize<'de> for BeveTagValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TagValueVisitor;

        impl<'de> Visitor<'de> for TagValueVisitor {
            type Value = BeveTagValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a BEVE string or unsigned-byte typed array")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BeveTagValue::Text(value.to_owned()))
            }

            fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BeveTagValue::Text(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BeveTagValue::Text(value))
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BeveTagValue::Hex(value.to_vec()))
            }

            fn visit_borrowed_bytes<E>(self, value: &'de [u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BeveTagValue::Hex(value.to_vec()))
            }

            fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BeveTagValue::Hex(value))
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut bytes = Vec::with_capacity(sequence.size_hint().unwrap_or_default());
                while let Some(byte) = sequence.next_element::<u8>()? {
                    bytes.push(byte);
                }
                Ok(BeveTagValue::Hex(bytes))
            }
        }

        deserializer.deserialize_any(TagValueVisitor)
    }
}

impl TryFrom<BeveEventOwned> for NostrEvent {
    type Error = BeveError;

    fn try_from(event: BeveEventOwned) -> Result<Self, Self::Error> {
        Ok(Self {
            id: event
                .id
                .try_into()
                .map_err(|_| BeveError::InvalidLength("id"))?,
            pubkey: event
                .pubkey
                .try_into()
                .map_err(|_| BeveError::InvalidLength("pubkey"))?,
            created_at: event.created_at,
            kind: event.kind,
            tags: event
                .tags
                .into_iter()
                .map(|tag| {
                    tag.into_iter()
                        .map(|value| match value {
                            BeveTagValue::Text(text) => text,
                            BeveTagValue::Hex(bytes) => hex::encode(bytes),
                        })
                        .collect()
                })
                .collect(),
            content: event.content,
            sig: event
                .sig
                .try_into()
                .map_err(|_| BeveError::InvalidLength("sig"))?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BeveError {
    #[error("BEVE decode error: {0}")]
    Decode(#[from] beve::Error),

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
    fn selective_access_and_full_verification() {
        let event = sample_event();
        let bytes = serialize(&event);
        verify(&bytes).unwrap();
        let (kind, pubkey) = read_kind_and_pubkey(&bytes).unwrap();
        assert_eq!(kind, event.kind);
        assert_eq!(pubkey, &event.pubkey);
    }

    #[test]
    fn trailing_data_is_rejected() {
        let mut bytes = serialize(&sample_event());
        bytes.push(0xff);
        assert!(deserialize(&bytes).is_err());
    }
}
