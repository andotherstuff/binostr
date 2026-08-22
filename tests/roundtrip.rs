//! Registry-wide semantic round-trip tests.

use binostr::stats::{self, Format};
use binostr::{EventLoader, NostrEvent};

fn event(index: u8, kind: u16, content: String, tags: Vec<Vec<String>>) -> NostrEvent {
    NostrEvent {
        id: [index; 32],
        pubkey: [index.wrapping_add(1); 32],
        created_at: if index == 9 {
            -86_400
        } else {
            1_700_000_000 + i64::from(index)
        },
        kind,
        tags,
        content,
        sig: [index.wrapping_add(2); 64],
    }
}

fn edge_cases() -> Vec<NostrEvent> {
    vec![
        event(1, 1, String::new(), vec![]),
        event(2, 0, r#"{"name":"test"}"#.into(), vec![]),
        event(
            3,
            1,
            "Hello 🌍! こんにちは 世界 🚀 émojis 中文".into(),
            vec![vec!["t".into(), "nostr".into()]],
        ),
        event(
            4,
            30023,
            "# Article\n\n".to_string() + &"Lorem ipsum. ".repeat(1000),
            vec![vec!["d".into(), "article".into()]],
        ),
        event(
            5,
            3,
            String::new(),
            (0..200)
                .map(|i| {
                    vec![
                        "p".into(),
                        hex::encode([i as u8; 32]),
                        "wss://relay.example".into(),
                    ]
                })
                .collect(),
        ),
        event(6, u16::MAX, "maximum kind".into(), vec![]),
        event(
            7,
            1,
            "Line1\nLine2\t\\\"\0".into(),
            vec![vec!["single".into()]],
        ),
        event(
            8,
            1,
            "abcdef1234567890".into(),
            vec![
                vec!["custom".into(), "DEADBEEF".into()],
                vec!["e".into(), hex::encode([0x11; 32]), String::new()],
            ],
        ),
        event(9, 1, "pre-epoch".into(), vec![]),
    ]
}

#[test]
fn every_format_roundtrips_edge_cases_and_batches() {
    let events = edge_cases();
    for &format in Format::all() {
        for (index, expected) in events.iter().enumerate() {
            let encoded = stats::serialize(expected, format);
            let actual = stats::deserialize(&encoded, format)
                .unwrap_or_else(|error| panic!("{} edge case {index}: {error}", format.name()));
            assert_eq!(*expected, actual, "{} edge case {index}", format.name());
        }
        let encoded = stats::serialize_batch(&events, format);
        let actual = stats::deserialize_batch(&encoded, format)
            .unwrap_or_else(|error| panic!("{} batch: {error}", format.name()));
        assert_eq!(events, actual, "{} batch", format.name());
    }
}

#[test]
fn every_format_roundtrips_real_corpus_events() {
    let events = EventLoader::open("data/sample.pb.gz")
        .expect("tracked sample corpus")
        .load_limited(100)
        .expect("sample corpus is readable");
    assert_eq!(events.len(), 100);
    for &format in Format::all() {
        for (index, expected) in events.iter().enumerate() {
            let encoded = stats::serialize(expected, format);
            let actual = stats::deserialize(&encoded, format)
                .unwrap_or_else(|error| panic!("{} real event {index}: {error}", format.name()));
            assert_eq!(*expected, actual, "{} real event {index}", format.name());
        }
    }
}
