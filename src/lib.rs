//! Binostr: Binary Nostr Serialization Benchmarks
//!
//! This library provides tools for benchmarking different serialization
//! formats for Nostr events: JSON, CBOR, Protocol Buffers, Cap'n Proto, and DannyPack.

pub mod capnp;
pub mod cbor;
pub mod config;
pub mod dannypack;
pub mod event;
pub mod json;
pub mod loader;
pub mod notepack;
pub mod proto;
pub mod sampler;
pub mod stats;

pub use event::NostrEvent;
pub use loader::EventLoader;
pub use sampler::{EventSampler, EXCLUDED_KINDS};

// Re-export generated protobuf types
pub mod proto_gen {
    pub mod nostr {
        include!(concat!(env!("OUT_DIR"), "/nostr.rs"));
    }
    pub mod nostr_binary {
        include!(concat!(env!("OUT_DIR"), "/nostr_binary.rs"));
    }
}
