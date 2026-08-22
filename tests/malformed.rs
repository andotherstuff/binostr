//! Decoder robustness checks over truncation and deterministic hostile bytes.

use std::panic::{catch_unwind, AssertUnwindSafe};

use binostr::stats::{self, Format};
use binostr::validation::EventLimits;
use binostr::NostrEvent;

fn event() -> NostrEvent {
    NostrEvent {
        id: [1; 32],
        pubkey: [2; 32],
        created_at: 1_700_000_000,
        kind: 1,
        tags: vec![vec!["p".into(), hex::encode([3; 32])]],
        content: "robustness 🧪".into(),
        sig: [4; 64],
    }
}

#[test]
fn no_decoder_panics_on_any_truncation() {
    for &format in Format::all() {
        let encoded = stats::serialize(&event(), format);
        for end in 0..encoded.len() {
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                stats::deserialize(&encoded[..end], format)
            }));
            assert!(
                outcome.is_ok(),
                "{} panicked at truncation {end}/{}",
                format.name(),
                encoded.len()
            );
        }
    }
}

#[test]
fn no_decoder_panics_on_deterministic_hostile_inputs() {
    let mut state = 0x9e37_79b9_u32;
    for &format in Format::all() {
        for length in 0..256 {
            let mut bytes = vec![0_u8; length];
            for byte in &mut bytes {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                *byte = state as u8;
            }
            let outcome = catch_unwind(AssertUnwindSafe(|| stats::deserialize(&bytes, format)));
            assert!(
                outcome.is_ok(),
                "{} panicked on {length} hostile bytes",
                format.name()
            );
        }
    }
}

#[test]
fn outer_limit_rejects_before_codec_decode() {
    let limits = EventLimits {
        max_wire_bytes: 8,
        ..EventLimits::default()
    };
    for &format in Format::all() {
        let encoded = stats::serialize(&event(), format);
        let error = stats::deserialize_limited(&encoded, format, &limits).unwrap_err();
        assert!(error.contains("wire payload"), "{}: {error}", format.name());
    }
}
