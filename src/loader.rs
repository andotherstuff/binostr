//! Event loader for .pb.gz files
//!
//! Loads Nostr events from length-delimited protobuf files compressed with gzip.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use flate2::read::GzDecoder;
use prost::Message;
use thiserror::Error;

use crate::event::NostrEvent;
use crate::proto_gen::nostr::ProtoEvent;

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),

    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("Invalid data: {0}")]
    InvalidData(String),
}

/// Loader for .pb.gz event files
pub struct EventLoader {
    reader: BufReader<GzDecoder<File>>,
    buffer: Vec<u8>,
}

impl EventLoader {
    /// Open a .pb.gz file for reading
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, LoadError> {
        let file = File::open(path)?;
        let decoder = GzDecoder::new(file);
        let reader = BufReader::with_capacity(1024 * 1024, decoder); // 1MB buffer

        Ok(Self {
            reader,
            buffer: Vec::with_capacity(64 * 1024), // 64KB initial capacity
        })
    }

    /// Read the next event from the file
    ///
    /// Returns None when EOF is reached
    pub fn next_event(&mut self) -> Result<Option<NostrEvent>, LoadError> {
        // Read varint length prefix
        let len = match self.read_varint() {
            Ok(Some(len)) => len as usize,
            Ok(None) => return Ok(None), // EOF
            Err(e) => return Err(e),
        };

        // Resize buffer if needed
        if self.buffer.len() < len {
            self.buffer.resize(len, 0);
        }

        // Read the message bytes
        self.reader.read_exact(&mut self.buffer[..len])?;

        // Decode protobuf
        let proto_event = ProtoEvent::decode(&self.buffer[..len])?;

        // Convert to NostrEvent
        let event = proto_to_event(proto_event)?;

        Ok(Some(event))
    }

    /// Read a varint from the stream
    fn read_varint(&mut self) -> Result<Option<u64>, LoadError> {
        let mut result: u64 = 0;
        let mut shift = 0;
        let mut byte = [0u8; 1];

        loop {
            match self.reader.read_exact(&mut byte) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    if shift == 0 {
                        return Ok(None); // Clean EOF
                    } else {
                        return Err(LoadError::InvalidData("Truncated varint".to_string()));
                    }
                }
                Err(e) => return Err(e.into()),
            }

            result |= ((byte[0] & 0x7F) as u64) << shift;

            if byte[0] & 0x80 == 0 {
                break;
            }

            shift += 7;
            if shift >= 64 {
                return Err(LoadError::InvalidData("Varint too long".to_string()));
            }
        }

        Ok(Some(result))
    }

    /// Load all events from the file into a vector
    pub fn load_all(mut self) -> Result<Vec<NostrEvent>, LoadError> {
        let mut events = Vec::new();
        while let Some(event) = self.next_event()? {
            events.push(event);
        }
        Ok(events)
    }

    /// Load up to `limit` events from the file
    pub fn load_limited(mut self, limit: usize) -> Result<Vec<NostrEvent>, LoadError> {
        let mut events = Vec::with_capacity(limit);
        while events.len() < limit {
            match self.next_event()? {
                Some(event) => events.push(event),
                None => break,
            }
        }
        Ok(events)
    }
}

impl Iterator for EventLoader {
    type Item = Result<NostrEvent, LoadError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_event() {
            Ok(Some(event)) => Some(Ok(event)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/// Convert a ProtoEvent to a NostrEvent
fn proto_to_event(proto: ProtoEvent) -> Result<NostrEvent, LoadError> {
    let id = hex::decode(&proto.id)?;
    let pubkey = hex::decode(&proto.pubkey)?;
    let sig = hex::decode(&proto.sig)?;

    let id: [u8; 32] = id
        .try_into()
        .map_err(|_| LoadError::InvalidData("Invalid id length".to_string()))?;
    let pubkey: [u8; 32] = pubkey
        .try_into()
        .map_err(|_| LoadError::InvalidData("Invalid pubkey length".to_string()))?;
    let sig: [u8; 64] = sig
        .try_into()
        .map_err(|_| LoadError::InvalidData("Invalid sig length".to_string()))?;

    let tags: Vec<Vec<String>> = proto.tags.into_iter().map(|t| t.values).collect();

    Ok(NostrEvent {
        id,
        pubkey,
        created_at: proto.created_at,
        kind: proto.kind as u16,
        tags,
        content: proto.content,
        sig,
    })
}

/// Load events from multiple .pb.gz files
pub fn load_from_directory<P: AsRef<Path>>(dir: P) -> Result<Vec<NostrEvent>, LoadError> {
    let mut events = Vec::new();
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "gz"))
        .collect();
    files.sort();

    for path in files {
        let loader = EventLoader::open(path)?;
        events.extend(loader.load_all()?);
    }

    Ok(events)
}

/// Load limited events from multiple .pb.gz files (round-robin)
pub fn load_limited_from_directory<P: AsRef<Path>>(
    dir: P,
    limit: usize,
) -> Result<Vec<NostrEvent>, LoadError> {
    let mut events = Vec::with_capacity(limit);
    let mut files: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "gz"))
        .map(|e| e.path())
        .collect();

    files.sort();

    if files.is_empty() {
        return Ok(events);
    }

    let per_file = (limit / files.len()).max(1);

    for path in files {
        if events.len() >= limit {
            break;
        }

        let remaining = limit - events.len();
        let to_load = per_file.min(remaining);

        let loader = EventLoader::open(&path)?;
        events.extend(loader.load_limited(to_load)?);
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_events() {
        let loader = EventLoader::open("data/sample.pb.gz").unwrap();
        let events = loader.load_limited(10).unwrap();
        assert_eq!(events.len(), 10);

        for event in &events {
            assert!(!event.id_hex().is_empty());
            assert!(!event.pubkey_hex().is_empty());
            assert!(!event.sig_hex().is_empty());
        }
    }
}
