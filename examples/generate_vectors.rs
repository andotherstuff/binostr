//! Generate deterministic wire vectors for independent implementations.

use std::collections::BTreeMap;
use std::fs;

use binostr::stats::{self, Format, FormatClass};
use binostr::NostrEvent;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
struct SemanticEvent {
    id: String,
    pubkey: String,
    created_at: i64,
    kind: u16,
    tags: Vec<Vec<String>>,
    content: String,
    sig: String,
}

#[derive(Serialize)]
struct Vector {
    bytes_hex: String,
    bytes_sha256: String,
}

#[derive(Serialize)]
struct Vectors {
    schema_version: u32,
    note: &'static str,
    semantic_event: SemanticEvent,
    formats: BTreeMap<&'static str, Vector>,
}

fn main() {
    let event = NostrEvent {
        id: std::array::from_fn(|index| index as u8),
        pubkey: std::array::from_fn(|index| (index + 32) as u8),
        created_at: 1_700_000_123,
        kind: 30_023,
        tags: vec![
            vec![
                "e".into(),
                hex::encode([0xab; 32]),
                "wss://relay.example".into(),
            ],
            vec!["custom".into(), "DEADBEEF".into(), "deadbeef".into()],
            vec![],
        ],
        content: "Hello, Nostr 🌍".into(),
        sig: std::array::from_fn(|index| (index + 64) as u8),
    };
    let semantic_event = SemanticEvent {
        id: hex::encode(event.id),
        pubkey: hex::encode(event.pubkey),
        created_at: event.created_at,
        kind: event.kind,
        tags: event.tags.clone(),
        content: event.content.clone(),
        sig: hex::encode(event.sig),
    };
    let mut formats = BTreeMap::new();
    for &format in Format::all() {
        if format == Format::CapnProto {
            continue;
        }
        if !matches!(
            format.class(),
            FormatClass::NostrBaseline
                | FormatClass::InteroperableCandidate
                | FormatClass::StandardEnvelopeCustomProfile
        ) {
            continue;
        }
        let bytes = stats::serialize(&event, format);
        assert_eq!(event, stats::deserialize(&bytes, format).unwrap());
        formats.insert(
            format.config_key(),
            Vector {
                bytes_hex: hex::encode(&bytes),
                bytes_sha256: hex::encode(Sha256::digest(&bytes)),
            },
        );
    }
    fs::create_dir_all("vectors").unwrap();
    fs::write(
        "vectors/interop-v1.json",
        serde_json::to_vec_pretty(&Vectors {
            schema_version: 1,
            note: "Cryptographic bytes are deterministic fixtures, not a valid signed event.",
            semantic_event,
            formats,
        })
        .unwrap(),
    )
    .unwrap();
    println!("Wrote vectors/interop-v1.json");
}
