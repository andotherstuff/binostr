//! NIP-01 semantic and cryptographic validation.

use secp256k1::{schnorr::Signature, Secp256k1, XOnlyPublicKey};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::NostrEvent;

#[derive(Debug, Clone, Copy)]
pub struct EventLimits {
    pub max_wire_bytes: usize,
    pub max_content_bytes: usize,
    pub max_tags: usize,
    pub max_tag_values: usize,
    pub max_tag_value_bytes: usize,
}

impl Default for EventLimits {
    fn default() -> Self {
        Self {
            max_wire_bytes: 1 << 20,
            max_content_bytes: 1 << 20,
            max_tags: 10_000,
            max_tag_values: 64,
            max_tag_value_bytes: 1 << 16,
        }
    }
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("wire payload is {actual} bytes; limit is {limit}")]
    WireTooLarge { actual: usize, limit: usize },
    #[error("content is {actual} bytes; limit is {limit}")]
    ContentTooLarge { actual: usize, limit: usize },
    #[error("event has {actual} tags; limit is {limit}")]
    TooManyTags { actual: usize, limit: usize },
    #[error("tag {tag} has {actual} values; limit is {limit}")]
    TooManyTagValues {
        tag: usize,
        actual: usize,
        limit: usize,
    },
    #[error("tag {tag} value {value} is {actual} bytes; limit is {limit}")]
    TagValueTooLarge {
        tag: usize,
        value: usize,
        actual: usize,
        limit: usize,
    },
    #[error("event id does not match the canonical NIP-01 serialization")]
    InvalidId,
    #[error("invalid x-only public key: {0}")]
    InvalidPublicKey(secp256k1::Error),
    #[error("invalid BIP-340 signature")]
    InvalidSignature,
}

pub fn check_wire_size(data: &[u8], limits: &EventLimits) -> Result<(), ValidationError> {
    if data.len() > limits.max_wire_bytes {
        return Err(ValidationError::WireTooLarge {
            actual: data.len(),
            limit: limits.max_wire_bytes,
        });
    }
    Ok(())
}

pub fn check_structure(event: &NostrEvent, limits: &EventLimits) -> Result<(), ValidationError> {
    if event.content.len() > limits.max_content_bytes {
        return Err(ValidationError::ContentTooLarge {
            actual: event.content.len(),
            limit: limits.max_content_bytes,
        });
    }
    if event.tags.len() > limits.max_tags {
        return Err(ValidationError::TooManyTags {
            actual: event.tags.len(),
            limit: limits.max_tags,
        });
    }
    for (tag_index, tag) in event.tags.iter().enumerate() {
        if tag.len() > limits.max_tag_values {
            return Err(ValidationError::TooManyTagValues {
                tag: tag_index,
                actual: tag.len(),
                limit: limits.max_tag_values,
            });
        }
        for (value_index, value) in tag.iter().enumerate() {
            if value.len() > limits.max_tag_value_bytes {
                return Err(ValidationError::TagValueTooLarge {
                    tag: tag_index,
                    value: value_index,
                    actual: value.len(),
                    limit: limits.max_tag_value_bytes,
                });
            }
        }
    }
    Ok(())
}

/// Compute the NIP-01 event identifier from `[0, pubkey, created_at, kind, tags, content]`.
pub fn compute_id(event: &NostrEvent) -> [u8; 32] {
    let canonical = serde_json::to_vec(&(
        0,
        hex::encode(event.pubkey),
        event.created_at,
        event.kind,
        &event.tags,
        &event.content,
    ))
    .expect("serializing a Nostr event into JSON cannot fail");
    Sha256::digest(canonical).into()
}

pub fn verify_id(event: &NostrEvent) -> Result<(), ValidationError> {
    if compute_id(event) == event.id {
        Ok(())
    } else {
        Err(ValidationError::InvalidId)
    }
}

pub fn verify_signature(event: &NostrEvent) -> Result<(), ValidationError> {
    let pubkey =
        XOnlyPublicKey::from_byte_array(event.pubkey).map_err(ValidationError::InvalidPublicKey)?;
    let signature = Signature::from_byte_array(event.sig);
    Secp256k1::verification_only()
        .verify_schnorr(&signature, &event.id, &pubkey)
        .map_err(|_| ValidationError::InvalidSignature)
}

pub fn verify_id_and_signature(event: &NostrEvent) -> Result<(), ValidationError> {
    verify_id(event)?;
    verify_signature(event)
}

pub fn validate(event: &NostrEvent, limits: &EventLimits) -> Result<(), ValidationError> {
    check_structure(event, limits)?;
    verify_id_and_signature(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventLoader;

    #[test]
    fn tracked_corpus_has_valid_ids_and_signatures() {
        let events = EventLoader::open("data/sample.pb.gz")
            .unwrap()
            .load_limited(10)
            .unwrap();
        assert_eq!(events.len(), 10);
        for event in events {
            validate(&event, &EventLimits::default()).unwrap();
        }
    }

    #[test]
    fn mutation_is_rejected() {
        let mut event = EventLoader::open("data/sample.pb.gz")
            .unwrap()
            .load_limited(1)
            .unwrap()
            .remove(0);
        event.content.push('!');
        assert!(matches!(verify_id(&event), Err(ValidationError::InvalidId)));
    }
}
