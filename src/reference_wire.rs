//! Shared Serde model for Rust-only reference codecs.

use serde::de::{self, Visitor};
use serde::ser::{SerializeSeq, SerializeTuple};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use crate::encoding::decode_lower_hex;
use crate::event::NostrEvent;

#[derive(Serialize)]
pub(crate) struct EventRef<'a> {
    id: FixedRef<'a, 32>,
    pubkey: FixedRef<'a, 32>,
    created_at: i64,
    kind: u16,
    tags: TagsRef<'a>,
    content: &'a str,
    sig: FixedRef<'a, 64>,
}

impl<'a> From<&'a NostrEvent> for EventRef<'a> {
    fn from(event: &'a NostrEvent) -> Self {
        Self {
            id: FixedRef(&event.id),
            pubkey: FixedRef(&event.pubkey),
            created_at: event.created_at,
            kind: event.kind,
            tags: TagsRef(&event.tags),
            content: &event.content,
            sig: FixedRef(&event.sig),
        }
    }
}

struct FixedRef<'a, const N: usize>(&'a [u8; N]);

impl<const N: usize> Serialize for FixedRef<'_, N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(N)?;
        for byte in self.0 {
            tuple.serialize_element(byte)?;
        }
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
        let mut values = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            values.serialize_element(&TagValueRef(value))?;
        }
        values.end()
    }
}

impl Serialize for TagValueRef<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match decode_lower_hex(self.0) {
            Some(bytes) => serializer.serialize_newtype_variant(
                "TagValue",
                1,
                "Hex",
                &serde_bytes::Bytes::new(&bytes),
            ),
            None => serializer.serialize_newtype_variant("TagValue", 0, "Text", self.0),
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct EventOwned {
    id: FixedOwned<32>,
    pubkey: FixedOwned<32>,
    created_at: i64,
    kind: u16,
    tags: Vec<Vec<TagValueOwned>>,
    content: String,
    sig: FixedOwned<64>,
}

struct FixedOwned<const N: usize>([u8; N]);

impl<'de, const N: usize> Deserialize<'de> for FixedOwned<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FixedVisitor<const N: usize>;

        impl<'de, const N: usize> Visitor<'de> for FixedVisitor<N> {
            type Value = FixedOwned<N>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "exactly {N} bytes")
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut bytes = [0_u8; N];
                for (index, byte) in bytes.iter_mut().enumerate() {
                    *byte = sequence
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(index, &self))?;
                }
                Ok(FixedOwned(bytes))
            }
        }

        deserializer.deserialize_tuple(N, FixedVisitor::<N>)
    }
}

#[derive(Deserialize)]
enum TagValueOwned {
    Text(String),
    Hex(#[serde(deserialize_with = "deserialize_bytes")] Vec<u8>),
}

fn deserialize_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    struct BytesVisitor;

    impl<'de> Visitor<'de> for BytesVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte string")
        }

        fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value.to_vec())
        }

        fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut bytes = Vec::with_capacity(sequence.size_hint().unwrap_or_default());
            while let Some(byte) = sequence.next_element()? {
                bytes.push(byte);
            }
            Ok(bytes)
        }
    }

    deserializer.deserialize_bytes(BytesVisitor)
}

impl From<EventOwned> for NostrEvent {
    fn from(event: EventOwned) -> Self {
        Self {
            id: event.id.0,
            pubkey: event.pubkey.0,
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
            sig: event.sig.0,
        }
    }
}
