//! FlatBuffers encoding for Nostr events.
//!
//! This uses generated code from `docs/nostr.fbs`. Decoding is verified before
//! any field is exposed. `read_*_trusted` is provided only for benchmarks that
//! model repeated access after a buffer has already passed verification.

use flatbuffers::{FlatBufferBuilder, WIPOffset};

use crate::encoding::decode_lower_hex;
use crate::event::NostrEvent;
use crate::nostr_generated::binostr as fb;

fn fixed<const N: usize>(
    value: flatbuffers::Vector<'_, u8>,
    name: &'static str,
) -> Result<[u8; N], FlatBuffersError> {
    let value: &[u8; N] = value
        .bytes()
        .try_into()
        .map_err(|_| FlatBuffersError::InvalidLength(name))?;
    Ok(*value)
}

fn build_event<'fbb>(
    builder: &mut FlatBufferBuilder<'fbb>,
    event: &NostrEvent,
) -> WIPOffset<fb::Event<'fbb>> {
    let id = builder.create_vector(&event.id);
    let pubkey = builder.create_vector(&event.pubkey);
    let sig = builder.create_vector(&event.sig);
    let content = builder.create_string(&event.content);

    let mut tag_offsets = Vec::with_capacity(event.tags.len());
    for tag in &event.tags {
        let mut value_offsets = Vec::with_capacity(tag.len());
        for value in tag {
            let args = if let Some(bytes) = decode_lower_hex(value) {
                fb::TagValueArgs {
                    text: None,
                    hex: Some(builder.create_vector(&bytes)),
                }
            } else {
                fb::TagValueArgs {
                    text: Some(builder.create_string(value)),
                    hex: None,
                }
            };
            value_offsets.push(fb::TagValue::create(builder, &args));
        }
        let values = builder.create_vector(&value_offsets);
        tag_offsets.push(fb::Tag::create(
            builder,
            &fb::TagArgs {
                values: Some(values),
            },
        ));
    }
    let tags = builder.create_vector(&tag_offsets);

    fb::Event::create(
        builder,
        &fb::EventArgs {
            id: Some(id),
            pubkey: Some(pubkey),
            created_at: event.created_at,
            kind: event.kind,
            tags: Some(tags),
            content: Some(content),
            sig: Some(sig),
        },
    )
}

fn event_from_view(event: fb::Event<'_>) -> Result<NostrEvent, FlatBuffersError> {
    let id = fixed(event.id(), "id")?;
    let pubkey = fixed(event.pubkey(), "pubkey")?;
    let sig = fixed(event.sig(), "sig")?;

    let tags = event
        .tags()
        .map(|tags| {
            tags.iter()
                .map(|tag| {
                    tag.values()
                        .ok_or(FlatBuffersError::MissingField("tag.values"))?
                        .iter()
                        .map(|value| match (value.text(), value.hex()) {
                            (Some(text), None) => Ok(text.to_owned()),
                            (None, Some(bytes)) => Ok(hex::encode(bytes.bytes())),
                            _ => Err(FlatBuffersError::InvalidTagValue),
                        })
                        .collect()
                })
                .collect()
        })
        .transpose()?
        .unwrap_or_default();

    Ok(NostrEvent {
        id,
        pubkey,
        created_at: event.created_at(),
        kind: event.kind(),
        tags,
        content: event.content().to_owned(),
        sig,
    })
}

pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let root = build_event(&mut builder, event);
    fb::finish_event_buffer(&mut builder, root);
    builder.finished_data().to_vec()
}

pub fn deserialize(data: &[u8]) -> Result<NostrEvent, FlatBuffersError> {
    event_from_view(fb::root_as_event(data)?)
}

pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let offsets: Vec<_> = events
        .iter()
        .map(|event| build_event(&mut builder, event))
        .collect();
    let events = builder.create_vector(&offsets);
    let root = fb::EventBatch::create(
        &mut builder,
        &fb::EventBatchArgs {
            events: Some(events),
        },
    );
    builder.finish(root, None);
    builder.finished_data().to_vec()
}

pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, FlatBuffersError> {
    let batch = flatbuffers::root::<fb::EventBatch<'_>>(data)?;
    batch
        .events()
        .ok_or(FlatBuffersError::MissingField("events"))?
        .iter()
        .map(event_from_view)
        .collect()
}

pub fn read_kind(data: &[u8]) -> Result<u16, FlatBuffersError> {
    Ok(fb::root_as_event(data)?.kind())
}

pub fn read_pubkey(data: &[u8]) -> Result<[u8; 32], FlatBuffersError> {
    fixed(fb::root_as_event(data)?.pubkey(), "pubkey")
}

pub fn read_kind_and_pubkey(data: &[u8]) -> Result<(u16, [u8; 32]), FlatBuffersError> {
    let event = fb::root_as_event(data)?;
    let pubkey = fixed(event.pubkey(), "pubkey")?;
    Ok((event.kind(), pubkey))
}

/// Read fields after `verify` has already succeeded for this exact buffer.
///
/// # Safety
/// The caller must not mutate `data` after verification.
pub unsafe fn read_kind_and_pubkey_trusted(data: &[u8]) -> (u16, [u8; 32]) {
    let event = unsafe { fb::root_as_event_unchecked(data) };
    let pubkey = fixed(event.pubkey(), "pubkey").expect("verified FlatBuffer has a 32-byte pubkey");
    (event.kind(), pubkey)
}

pub fn verify(data: &[u8]) -> Result<(), FlatBuffersError> {
    fb::root_as_event(data)?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum FlatBuffersError {
    #[error("invalid FlatBuffer: {0}")]
    Invalid(#[from] flatbuffers::InvalidFlatbuffer),

    #[error("missing FlatBuffers field: {0}")]
    MissingField(&'static str),

    #[error("invalid {0} length")]
    InvalidLength(&'static str),

    #[error("tag value must contain exactly one of text or hex")]
    InvalidTagValue,
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
        let bytes = serialize(&event);
        assert_eq!(deserialize(&bytes).unwrap(), event);
    }

    #[test]
    fn batch_roundtrip() {
        let events = vec![event(), event()];
        assert_eq!(
            deserialize_batch(&serialize_batch(&events)).unwrap(),
            events
        );
    }

    #[test]
    fn selective_access() {
        let event = event();
        let bytes = serialize(&event);
        assert_eq!(
            read_kind_and_pubkey(&bytes).unwrap(),
            (event.kind, event.pubkey)
        );
    }
}
