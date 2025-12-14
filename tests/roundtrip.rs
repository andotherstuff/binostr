//! Roundtrip integration tests
//!
//! Tests that all serialization formats correctly roundtrip real Nostr events
//! without data loss or corruption.

use binostr::{capnp, cbor, dannypack, json, notepack, proto, EventLoader, NostrEvent};

/// Load real events from the sample data file
fn load_real_events(count: usize) -> Vec<NostrEvent> {
    match EventLoader::open("data/sample.pb.gz") {
        Ok(loader) => loader.load_limited(count).unwrap_or_default(),
        Err(e) => {
            eprintln!("Warning: Could not load sample data: {}", e);
            Vec::new()
        }
    }
}

/// Generate a variety of edge case events for testing
fn generate_edge_case_events() -> Vec<NostrEvent> {
    vec![
        // Empty content
        NostrEvent {
            id: [0x11; 32],
            pubkey: [0x22; 32],
            created_at: 1700000000,
            kind: 1,
            tags: vec![vec!["p".to_string(), hex::encode([0xab; 32])]],
            content: String::new(),
            sig: [0x33; 64],
        },
        // Empty tags
        NostrEvent {
            id: [0x44; 32],
            pubkey: [0x55; 32],
            created_at: 1700000001,
            kind: 0,
            tags: vec![],
            content: r#"{"name":"test"}"#.to_string(),
            sig: [0x66; 64],
        },
        // Unicode content with emojis
        NostrEvent {
            id: [0x77; 32],
            pubkey: [0x88; 32],
            created_at: 1700000002,
            kind: 1,
            tags: vec![vec!["t".to_string(), "nostr".to_string()]],
            content: "Hello 🌍! こんにちは 世界 🚀 émojis 中文".to_string(),
            sig: [0x99; 64],
        },
        // Very long content (simulated article)
        NostrEvent {
            id: [0xaa; 32],
            pubkey: [0xbb; 32],
            created_at: 1700000003,
            kind: 30023,
            tags: vec![
                vec!["d".to_string(), "test-article".to_string()],
                vec!["title".to_string(), "Test Article".to_string()],
            ],
            content: "# Long Article\n\n".to_string() + &"Lorem ipsum dolor sit amet. ".repeat(1000),
            sig: [0xcc; 64],
        },
        // Many tags (simulated follow list)
        NostrEvent {
            id: [0xdd; 32],
            pubkey: [0xee; 32],
            created_at: 1700000004,
            kind: 3,
            tags: (0..200)
                .map(|i| {
                    vec![
                        "p".to_string(),
                        hex::encode([i as u8; 32]),
                        "wss://relay.example.com".to_string(),
                    ]
                })
                .collect(),
            content: String::new(),
            sig: [0xff; 64],
        },
        // Maximum kind value
        NostrEvent {
            id: [0x12; 32],
            pubkey: [0x34; 32],
            created_at: 1700000005,
            kind: 65535,
            tags: vec![],
            content: "Max kind event".to_string(),
            sig: [0x56; 64],
        },
        // Minimum timestamp (Unix epoch)
        NostrEvent {
            id: [0x78; 32],
            pubkey: [0x9a; 32],
            created_at: 0,
            kind: 1,
            tags: vec![],
            content: "Epoch event".to_string(),
            sig: [0xbc; 64],
        },
        // Negative timestamp (pre-1970, theoretical)
        NostrEvent {
            id: [0xde; 32],
            pubkey: [0xf0; 32],
            created_at: -86400, // One day before epoch
            kind: 1,
            tags: vec![],
            content: "Pre-epoch event".to_string(),
            sig: [0x13; 64],
        },
        // Content with JSON escaping characters
        NostrEvent {
            id: [0x24; 32],
            pubkey: [0x35; 32],
            created_at: 1700000006,
            kind: 1,
            tags: vec![],
            content: "Line1\nLine2\tTabbed\r\nWindows\\ \"quoted\" \u{0000}null".to_string(),
            sig: [0x46; 64],
        },
        // Hex-looking content (tests CBOR/DannyPack hex detection)
        NostrEvent {
            id: [0x57; 32],
            pubkey: [0x68; 32],
            created_at: 1700000007,
            kind: 1,
            tags: vec![],
            content: "abcdef1234567890".to_string(), // Valid hex string
            sig: [0x79; 64],
        },
        // Tags with various value types
        NostrEvent {
            id: [0x8a; 32],
            pubkey: [0x9b; 32],
            created_at: 1700000008,
            kind: 1,
            tags: vec![
                vec!["e".to_string(), hex::encode([0x11; 32]), "".to_string()], // Empty relay hint
                vec!["p".to_string(), hex::encode([0x22; 32])],
                vec!["t".to_string(), "hashtag".to_string()],
                vec!["r".to_string(), "https://example.com".to_string()],
                vec![
                    "a".to_string(),
                    "30023:abc:def".to_string(),
                    "wss://relay.example.com".to_string(),
                ],
                vec!["single".to_string()], // Single-element tag
            ],
            content: "Event with various tags".to_string(),
            sig: [0xac; 64],
        },
        // Reaction event (tiny)
        NostrEvent {
            id: [0xbd; 32],
            pubkey: [0xce; 32],
            created_at: 1700000009,
            kind: 7,
            tags: vec![
                vec!["e".to_string(), hex::encode([0x33; 32])],
                vec!["p".to_string(), hex::encode([0x44; 32])],
            ],
            content: "🤙".to_string(),
            sig: [0xdf; 64],
        },
    ]
}

// JSON tests
mod json_roundtrip {
    use super::*;

    #[test]
    fn roundtrip_edge_cases() {
        let events = generate_edge_case_events();
        for (i, event) in events.iter().enumerate() {
            let serialized = json::serialize(event);
            let deserialized = json::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize edge case {}: {}", i, e));
            assert_eq!(event, &deserialized, "Edge case {} roundtrip failed", i);
        }
    }

    #[test]
    fn roundtrip_real_events() {
        let events = load_real_events(100);
        if events.is_empty() {
            eprintln!("Skipping real events test - no sample data available");
            return;
        }

        for (i, event) in events.iter().enumerate() {
            let serialized = json::serialize(event);
            let deserialized = json::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize real event {}: {}", i, e));
            assert_eq!(
                event, &deserialized,
                "Real event {} roundtrip failed (kind={})",
                i, event.kind
            );
        }
    }

    #[test]
    fn batch_roundtrip() {
        let events = generate_edge_case_events();
        let serialized = json::serialize_batch(&events);
        let deserialized = json::deserialize_batch(&serialized).unwrap();
        assert_eq!(events, deserialized);
    }
}

// CBOR Schemaless tests
mod cbor_schemaless_roundtrip {
    use super::*;

    #[test]
    fn roundtrip_edge_cases() {
        let events = generate_edge_case_events();
        for (i, event) in events.iter().enumerate() {
            let serialized = cbor::schemaless::serialize(event);
            let deserialized = cbor::schemaless::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize edge case {}: {}", i, e));
            assert_eq!(event, &deserialized, "Edge case {} roundtrip failed", i);
        }
    }

    #[test]
    fn roundtrip_real_events() {
        let events = load_real_events(100);
        if events.is_empty() {
            eprintln!("Skipping real events test - no sample data available");
            return;
        }

        for (i, event) in events.iter().enumerate() {
            let serialized = cbor::schemaless::serialize(event);
            let deserialized = cbor::schemaless::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize real event {}: {}", i, e));
            assert_eq!(
                event, &deserialized,
                "Real event {} roundtrip failed (kind={})",
                i, event.kind
            );
        }
    }

    #[test]
    fn batch_roundtrip() {
        let events = generate_edge_case_events();
        let serialized = cbor::schemaless::serialize_batch(&events);
        let deserialized = cbor::schemaless::deserialize_batch(&serialized).unwrap();
        assert_eq!(events, deserialized);
    }
}

// CBOR Packed tests
mod cbor_packed_roundtrip {
    use super::*;

    #[test]
    fn roundtrip_edge_cases() {
        let events = generate_edge_case_events();
        for (i, event) in events.iter().enumerate() {
            let serialized = cbor::packed::serialize(event);
            let deserialized = cbor::packed::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize edge case {}: {}", i, e));
            assert_eq!(event, &deserialized, "Edge case {} roundtrip failed", i);
        }
    }

    #[test]
    fn roundtrip_real_events() {
        let events = load_real_events(100);
        if events.is_empty() {
            eprintln!("Skipping real events test - no sample data available");
            return;
        }

        for (i, event) in events.iter().enumerate() {
            let serialized = cbor::packed::serialize(event);
            let deserialized = cbor::packed::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize real event {}: {}", i, e));
            assert_eq!(
                event, &deserialized,
                "Real event {} roundtrip failed (kind={})",
                i, event.kind
            );
        }
    }

    #[test]
    fn batch_roundtrip() {
        let events = generate_edge_case_events();
        let serialized = cbor::packed::serialize_batch(&events);
        let deserialized = cbor::packed::deserialize_batch(&serialized).unwrap();
        assert_eq!(events, deserialized);
    }
}

// CBOR IntKey tests
mod cbor_intkey_roundtrip {
    use super::*;

    #[test]
    fn roundtrip_edge_cases() {
        let events = generate_edge_case_events();
        for (i, event) in events.iter().enumerate() {
            let serialized = cbor::intkey::serialize(event);
            let deserialized = cbor::intkey::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize edge case {}: {}", i, e));
            assert_eq!(event, &deserialized, "Edge case {} roundtrip failed", i);
        }
    }

    #[test]
    fn roundtrip_real_events() {
        let events = load_real_events(100);
        if events.is_empty() {
            eprintln!("Skipping real events test - no sample data available");
            return;
        }

        for (i, event) in events.iter().enumerate() {
            let serialized = cbor::intkey::serialize(event);
            let deserialized = cbor::intkey::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize real event {}: {}", i, e));
            assert_eq!(
                event, &deserialized,
                "Real event {} roundtrip failed (kind={})",
                i, event.kind
            );
        }
    }

    #[test]
    fn batch_roundtrip() {
        let events = generate_edge_case_events();
        let serialized = cbor::intkey::serialize_batch(&events);
        let deserialized = cbor::intkey::deserialize_batch(&serialized).unwrap();
        assert_eq!(events, deserialized);
    }
}

// Proto String tests
mod proto_string_roundtrip {
    use super::*;

    #[test]
    fn roundtrip_edge_cases() {
        let events = generate_edge_case_events();
        for (i, event) in events.iter().enumerate() {
            let serialized = proto::string::serialize(event);
            let deserialized = proto::string::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize edge case {}: {}", i, e));
            assert_eq!(event, &deserialized, "Edge case {} roundtrip failed", i);
        }
    }

    #[test]
    fn roundtrip_real_events() {
        let events = load_real_events(100);
        if events.is_empty() {
            eprintln!("Skipping real events test - no sample data available");
            return;
        }

        for (i, event) in events.iter().enumerate() {
            let serialized = proto::string::serialize(event);
            let deserialized = proto::string::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize real event {}: {}", i, e));
            assert_eq!(
                event, &deserialized,
                "Real event {} roundtrip failed (kind={})",
                i, event.kind
            );
        }
    }

    #[test]
    fn batch_roundtrip() {
        let events = generate_edge_case_events();
        let serialized = proto::string::serialize_batch(&events);
        let deserialized = proto::string::deserialize_batch(&serialized).unwrap();
        assert_eq!(events, deserialized);
    }
}

// Proto Binary tests
mod proto_binary_roundtrip {
    use super::*;

    #[test]
    fn roundtrip_edge_cases() {
        let events = generate_edge_case_events();
        for (i, event) in events.iter().enumerate() {
            let serialized = proto::binary::serialize(event);
            let deserialized = proto::binary::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize edge case {}: {}", i, e));
            assert_eq!(event, &deserialized, "Edge case {} roundtrip failed", i);
        }
    }

    #[test]
    fn roundtrip_real_events() {
        let events = load_real_events(100);
        if events.is_empty() {
            eprintln!("Skipping real events test - no sample data available");
            return;
        }

        for (i, event) in events.iter().enumerate() {
            let serialized = proto::binary::serialize(event);
            let deserialized = proto::binary::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize real event {}: {}", i, e));
            assert_eq!(
                event, &deserialized,
                "Real event {} roundtrip failed (kind={})",
                i, event.kind
            );
        }
    }

    #[test]
    fn batch_roundtrip() {
        let events = generate_edge_case_events();
        let serialized = proto::binary::serialize_batch(&events);
        let deserialized = proto::binary::deserialize_batch(&serialized).unwrap();
        assert_eq!(events, deserialized);
    }
}

// Cap'n Proto tests
mod capnp_roundtrip {
    use super::*;

    #[test]
    fn roundtrip_edge_cases() {
        let events = generate_edge_case_events();
        for (i, event) in events.iter().enumerate() {
            let serialized = capnp::serialize_event(event);
            let deserialized = capnp::deserialize_event(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize edge case {}: {}", i, e));
            assert_eq!(event, &deserialized, "Edge case {} roundtrip failed", i);
        }
    }

    #[test]
    fn roundtrip_real_events() {
        let events = load_real_events(100);
        if events.is_empty() {
            eprintln!("Skipping real events test - no sample data available");
            return;
        }

        for (i, event) in events.iter().enumerate() {
            let serialized = capnp::serialize_event(event);
            let deserialized = capnp::deserialize_event(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize real event {}: {}", i, e));
            assert_eq!(
                event, &deserialized,
                "Real event {} roundtrip failed (kind={})",
                i, event.kind
            );
        }
    }

    #[test]
    fn batch_roundtrip() {
        let events = generate_edge_case_events();
        let serialized = capnp::serialize_batch(&events);
        let deserialized = capnp::deserialize_batch(&serialized).unwrap();
        assert_eq!(events, deserialized);
    }
}

// Cap'n Proto Packed tests
mod capnp_packed_roundtrip {
    use super::*;

    #[test]
    fn roundtrip_edge_cases() {
        let events = generate_edge_case_events();
        for (i, event) in events.iter().enumerate() {
            let serialized = capnp::serialize_event_packed(event);
            let deserialized = capnp::deserialize_event_packed(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize edge case {}: {}", i, e));
            assert_eq!(event, &deserialized, "Edge case {} roundtrip failed", i);
        }
    }

    #[test]
    fn roundtrip_real_events() {
        let events = load_real_events(100);
        if events.is_empty() {
            eprintln!("Skipping real events test - no sample data available");
            return;
        }

        for (i, event) in events.iter().enumerate() {
            let serialized = capnp::serialize_event_packed(event);
            let deserialized = capnp::deserialize_event_packed(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize real event {}: {}", i, e));
            assert_eq!(
                event, &deserialized,
                "Real event {} roundtrip failed (kind={})",
                i, event.kind
            );
        }
    }

    #[test]
    fn batch_roundtrip() {
        let events = generate_edge_case_events();
        let serialized = capnp::serialize_batch_packed(&events);
        let deserialized = capnp::deserialize_batch_packed(&serialized).unwrap();
        assert_eq!(events, deserialized);
    }
}

// DannyPack tests
mod dannypack_roundtrip {
    use super::*;

    #[test]
    fn roundtrip_edge_cases() {
        let events = generate_edge_case_events();
        for (i, event) in events.iter().enumerate() {
            let serialized = dannypack::serialize_to_vec(event);
            let deserialized = dannypack::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize edge case {}: {}", i, e));
            assert_eq!(event, &deserialized, "Edge case {} roundtrip failed", i);
        }
    }

    #[test]
    fn roundtrip_real_events() {
        let events = load_real_events(100);
        if events.is_empty() {
            eprintln!("Skipping real events test - no sample data available");
            return;
        }

        for (i, event) in events.iter().enumerate() {
            let serialized = dannypack::serialize_to_vec(event);
            let deserialized = dannypack::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize real event {}: {}", i, e));
            assert_eq!(
                event, &deserialized,
                "Real event {} roundtrip failed (kind={})",
                i, event.kind
            );
        }
    }

    #[test]
    fn batch_roundtrip() {
        let events = generate_edge_case_events();
        let serialized = dannypack::serialize_batch(&events);
        let deserialized = dannypack::deserialize_batch(&serialized).unwrap();
        assert_eq!(events, deserialized);
    }
}

// Notepack tests
mod notepack_roundtrip {
    use super::*;

    #[test]
    fn roundtrip_edge_cases() {
        let events = generate_edge_case_events();
        for (i, event) in events.iter().enumerate() {
            let serialized = notepack::serialize(event);
            let deserialized = notepack::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize edge case {}: {}", i, e));
            assert_eq!(event, &deserialized, "Edge case {} roundtrip failed", i);
        }
    }

    #[test]
    fn roundtrip_real_events() {
        let events = load_real_events(100);
        if events.is_empty() {
            eprintln!("Skipping real events test - no sample data available");
            return;
        }

        for (i, event) in events.iter().enumerate() {
            let serialized = notepack::serialize(event);
            let deserialized = notepack::deserialize(&serialized)
                .unwrap_or_else(|e| panic!("Failed to deserialize real event {}: {}", i, e));
            assert_eq!(
                event, &deserialized,
                "Real event {} roundtrip failed (kind={})",
                i, event.kind
            );
        }
    }

    #[test]
    fn batch_roundtrip() {
        let events = generate_edge_case_events();
        let serialized = notepack::serialize_batch(&events);
        let deserialized = notepack::deserialize_batch(&serialized).unwrap();
        assert_eq!(events, deserialized);
    }
}

// Cross-format consistency tests
mod cross_format {
    use super::*;

    /// Verify all formats produce the same logical event
    #[test]
    fn all_formats_equivalent() {
        let events = generate_edge_case_events();

        for (i, original) in events.iter().enumerate() {
            // Serialize with each format
            let json_bytes = json::serialize(original);
            let cbor_schemaless_bytes = cbor::schemaless::serialize(original);
            let cbor_packed_bytes = cbor::packed::serialize(original);
            let cbor_intkey_bytes = cbor::intkey::serialize(original);
            let proto_string_bytes = proto::string::serialize(original);
            let proto_binary_bytes = proto::binary::serialize(original);
            let capnp_bytes = capnp::serialize_event(original);
            let capnp_packed_bytes = capnp::serialize_event_packed(original);
            let dannypack_bytes = dannypack::serialize_to_vec(original);
            let notepack_bytes = notepack::serialize(original);

            // Deserialize each
            let from_json = json::deserialize(&json_bytes).unwrap();
            let from_cbor_schemaless = cbor::schemaless::deserialize(&cbor_schemaless_bytes).unwrap();
            let from_cbor_packed = cbor::packed::deserialize(&cbor_packed_bytes).unwrap();
            let from_cbor_intkey = cbor::intkey::deserialize(&cbor_intkey_bytes).unwrap();
            let from_proto_string = proto::string::deserialize(&proto_string_bytes).unwrap();
            let from_proto_binary = proto::binary::deserialize(&proto_binary_bytes).unwrap();
            let from_capnp = capnp::deserialize_event(&capnp_bytes).unwrap();
            let from_capnp_packed = capnp::deserialize_event_packed(&capnp_packed_bytes).unwrap();
            let from_dannypack = dannypack::deserialize(&dannypack_bytes).unwrap();
            let from_notepack = notepack::deserialize(&notepack_bytes).unwrap();

            // All should equal original
            assert_eq!(original, &from_json, "JSON mismatch at event {}", i);
            assert_eq!(original, &from_cbor_schemaless, "CBOR Schemaless mismatch at event {}", i);
            assert_eq!(original, &from_cbor_packed, "CBOR Packed mismatch at event {}", i);
            assert_eq!(original, &from_cbor_intkey, "CBOR IntKey mismatch at event {}", i);
            assert_eq!(original, &from_proto_string, "Proto String mismatch at event {}", i);
            assert_eq!(original, &from_proto_binary, "Proto Binary mismatch at event {}", i);
            assert_eq!(original, &from_capnp, "Cap'n Proto mismatch at event {}", i);
            assert_eq!(original, &from_capnp_packed, "Cap'n Proto Packed mismatch at event {}", i);
            assert_eq!(original, &from_dannypack, "DannyPack mismatch at event {}", i);
            assert_eq!(original, &from_notepack, "Notepack mismatch at event {}", i);
        }
    }
}
