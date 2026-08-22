//! Shared standard-data-model profile used by packed CBOR and MessagePack.
//!
//! Keeping this representation shared prevents codec benchmarks from also
//! measuring different Rust-side modeling strategies.

use std::fmt;

use serde::de::{self, Visitor};
use serde::ser::{SerializeSeq, SerializeTuple};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::encoding::decode_lower_hex;
use crate::event::NostrEvent;

pub(crate) struct EventRef<'a> {
    event: &'a NostrEvent,
}

impl<'a> From<&'a NostrEvent> for EventRef<'a> {
    fn from(event: &'a NostrEvent) -> Self {
        Self { event }
    }
}

impl Serialize for EventRef<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let event = self.event;
        let mut tuple = serializer.serialize_tuple(7)?;
        tuple.serialize_element(serde_bytes::Bytes::new(&event.id))?;
        tuple.serialize_element(serde_bytes::Bytes::new(&event.pubkey))?;
        tuple.serialize_element(&event.created_at)?;
        tuple.serialize_element(&event.kind)?;
        tuple.serialize_element(&TagsRef(&event.tags))?;
        tuple.serialize_element(&event.content)?;
        tuple.serialize_element(serde_bytes::Bytes::new(&event.sig))?;
        tuple.end()
    }
}

pub(crate) struct EventsRef<'a>(pub(crate) &'a [NostrEvent]);

impl Serialize for EventsRef<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut events = serializer.serialize_seq(Some(self.0.len()))?;
        for event in self.0 {
            events.serialize_element(&EventRef::from(event))?;
        }
        events.end()
    }
}

struct TagsRef<'a>(&'a [Vec<String>]);
struct TagRef<'a>(&'a [String]);
struct TagValueRef<'a>(&'a str);

impl Serialize for TagsRef<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tags = serializer.serialize_seq(Some(self.0.len()))?;
        for tag in self.0 {
            tags.serialize_element(&TagRef(tag))?;
        }
        tags.end()
    }
}

impl Serialize for TagRef<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tag = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            tag.serialize_element(&TagValueRef(value))?;
        }
        tag.end()
    }
}

impl Serialize for TagValueRef<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match decode_lower_hex(self.0) {
            Some(bytes) => serializer.serialize_bytes(&bytes),
            None => serializer.serialize_str(self.0),
        }
    }
}

pub(crate) struct EventOwned {
    id: Vec<u8>,
    pubkey: Vec<u8>,
    created_at: i64,
    kind: u16,
    tags: Vec<Vec<TagValueOwned>>,
    content: String,
    sig: Vec<u8>,
}

impl<'de> Deserialize<'de> for EventOwned {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EventVisitor;

        impl<'de> Visitor<'de> for EventVisitor {
            type Value = EventOwned;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a seven-element positional Nostr event")
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                fn next<'de, A, T>(sequence: &mut A, index: usize) -> Result<T, A::Error>
                where
                    A: de::SeqAccess<'de>,
                    T: Deserialize<'de>,
                {
                    sequence
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(index, &EventVisitor))
                }

                let id: serde_bytes::ByteBuf = next(&mut sequence, 0)?;
                let pubkey: serde_bytes::ByteBuf = next(&mut sequence, 1)?;
                let created_at = next(&mut sequence, 2)?;
                let kind = next(&mut sequence, 3)?;
                let tags = next(&mut sequence, 4)?;
                let content = next(&mut sequence, 5)?;
                let sig: serde_bytes::ByteBuf = next(&mut sequence, 6)?;

                if sequence.next_element::<de::IgnoredAny>()?.is_some() {
                    return Err(de::Error::invalid_length(8, &EventVisitor));
                }

                Ok(EventOwned {
                    id: id.into_vec(),
                    pubkey: pubkey.into_vec(),
                    created_at,
                    kind,
                    tags,
                    content,
                    sig: sig.into_vec(),
                })
            }
        }

        deserializer.deserialize_tuple(7, EventVisitor)
    }
}

enum TagValueOwned {
    Text(String),
    Hex(Vec<u8>),
}

impl<'de> Deserialize<'de> for TagValueOwned {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TagValueVisitor;

        impl<'de> Visitor<'de> for TagValueVisitor {
            type Value = TagValueOwned;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a text or byte-string tag value")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(TagValueOwned::Text(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(TagValueOwned::Text(value))
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(TagValueOwned::Hex(value.to_vec()))
            }

            fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(TagValueOwned::Hex(value))
            }
        }

        deserializer.deserialize_any(TagValueVisitor)
    }
}

impl TryFrom<EventOwned> for NostrEvent {
    type Error = PositionalError;

    fn try_from(event: EventOwned) -> Result<Self, Self::Error> {
        Ok(Self {
            id: event
                .id
                .try_into()
                .map_err(|_| PositionalError::InvalidLength("id"))?,
            pubkey: event
                .pubkey
                .try_into()
                .map_err(|_| PositionalError::InvalidLength("pubkey"))?,
            created_at: event.created_at,
            kind: event.kind,
            tags: event
                .tags
                .into_iter()
                .map(|tag| {
                    tag.into_iter()
                        .map(|value| match value {
                            TagValueOwned::Text(text) => text,
                            TagValueOwned::Hex(bytes) => hex::encode(bytes),
                        })
                        .collect()
                })
                .collect(),
            content: event.content,
            sig: event
                .sig
                .try_into()
                .map_err(|_| PositionalError::InvalidLength("sig"))?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PositionalError {
    #[error("invalid {0} length")]
    InvalidLength(&'static str),
}
