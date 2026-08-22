//! Binostr: Binary Nostr Serialization Benchmarks
//!
//! This library provides a common benchmark and validation harness for NIP-01
//! JSON plus standards-based, schema-based, Rust-only, and Nostr-specific binary
//! representations. See [`stats::Format`] for the central registry and the
//! repository methodology for the exact wire profiles and comparison contract.

pub mod avro;
pub mod beve;
pub mod bincode;
pub mod bson;
pub mod capnp;
pub mod cbor;
pub mod config;
pub mod dannypack;
mod encoding;
pub mod event;
pub mod flatbuffers;
pub mod flexbuffers;
pub mod json;
pub mod loader;
pub mod msgpack;
pub mod notepack;
mod positional;
pub mod postcard;
pub mod proto;
mod reference_wire;
pub mod sampler;
pub mod stats;
pub mod thrift_compact;
pub mod validation;

#[doc(hidden)]
#[allow(clippy::all, warnings)]
pub mod nostr_generated;

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
