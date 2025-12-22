# Notepack Performance Optimizations

This document outlines proposed optimizations for the notepack library based on benchmarking against other binary formats (protobuf, Cap'n Proto, CBOR) in real-world Nostr relay workloads.

## Current Performance

Notepack achieves **excellent compression** (35% smaller than JSON), but serialization speed lags behind protobuf:

| Format | Serialize | Deserialize | Size vs JSON |
|--------|-----------|-------------|--------------|
| Proto Binary | 1.72 ms | 2.04 ms | 88.9% |
| **Notepack** | 4.08 ms | 2.98 ms | **64.6%** |
| JSON | 2.76 ms | 3.79 ms | 100% |

*Benchmarks: 1000 real Nostr events, release mode, Apple M-series*

Notepack wins on size but is **2.4x slower** than protobuf for serialization.

---

## Root Cause Analysis

### Serialization Bottleneck

The current `NoteBuf` API requires hex strings for id/pubkey/sig:

```rust
// Current usage - forces hex encoding
let note = NoteBuf {
    id: hex::encode(event.id),        // 🔴 32 bytes → 64 char String
    pubkey: hex::encode(event.pubkey), // 🔴 32 bytes → 64 char String
    sig: hex::encode(event.sig),       // 🔴 64 bytes → 128 char String
    tags: event.tags.clone(),          // 🔴 Clone entire tag structure
    content: event.content.clone(),    // 🔴 Clone content
    ...
};
```

**Overhead per event:**
- 3× hex encode operations (allocates 256 characters)
- 2× clone operations (tags + content)
- Intermediate `NoteBuf` struct allocation

Yet notepack's wire format stores these as **binary bytes internally** - the hex conversion is wasted work.

### Deserialization (Already Good)

The `Note` struct correctly returns binary references:

```rust
let note = parser.into_note()?;
let id: [u8; 32] = *note.id;  // ✅ Direct copy from binary
```

Main overhead is `.to_string()` calls for tags/content (unavoidable for owned data).

---

## Proposed Optimizations

### 1. Binary Serialization API

Add a new struct that accepts binary data directly:

```rust
/// Zero-copy serialization input - all references, no allocations
pub struct NoteBinary<'a> {
    pub id: &'a [u8; 32],
    pub pubkey: &'a [u8; 32],
    pub created_at: u64,
    pub kind: u64,
    pub tags: &'a [Vec<String>],
    pub content: &'a str,
    pub sig: &'a [u8; 64],
}

impl NoteBinary<'_> {
    /// Serialize to notepack binary format
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.estimated_size());
        self.pack_into(&mut buf);
        buf
    }

    /// Serialize into an existing buffer (avoids allocation)
    pub fn pack_into(&self, buf: &mut Vec<u8>) {
        // Write id directly as 32 bytes
        buf.extend_from_slice(self.id);
        // Write pubkey directly as 32 bytes
        buf.extend_from_slice(self.pubkey);
        // ... etc
    }

    /// Estimate serialized size for pre-allocation
    pub fn estimated_size(&self) -> usize {
        32 + 32 + 64  // id + pubkey + sig
        + 10          // created_at varint
        + 5           // kind varint
        + self.content.len() + 5
        + self.tags.iter().map(|t| t.iter().map(|s| s.len() + 2).sum::<usize>()).sum::<usize>()
    }
}
```

**Usage:**

```rust
// New fast path - zero allocations before pack
let note = NoteBinary {
    id: &event.id,
    pubkey: &event.pubkey,
    sig: &event.sig,
    tags: &event.tags,
    content: &event.content,
    created_at: event.created_at,
    kind: event.kind,
};
let bytes = note.pack();
```

**Expected improvement:** ~2-3x faster serialization

---

### 2. Zero-Copy Field Access

For relay filtering, allow reading specific fields without full deserialization:

```rust
impl<'a> NoteParser<'a> {
    /// Read just the kind field (for filtering by event type)
    pub fn read_kind(&self) -> Result<u64, Error> {
        // Seek to kind offset in the binary format
        // Read and return varint
    }

    /// Read just the pubkey (for filtering by author)
    pub fn read_pubkey(&self) -> Result<&[u8; 32], Error> {
        // Return slice pointing directly into source buffer
    }

    /// Read kind and pubkey together (common filter pattern)
    pub fn read_kind_and_pubkey(&self) -> Result<(u64, &[u8; 32]), Error> {
        // Single pass to get both
    }

    /// Read created_at for time-range queries
    pub fn read_created_at(&self) -> Result<u64, Error> {
        // Seek to timestamp offset
    }
}
```

**Use case:** Relay processing millions of stored events:

```rust
// Filter without deserializing tags/content/sig
for event_bytes in database.scan() {
    let parser = NoteParser::new(event_bytes);

    // Fast field access - no allocation
    let kind = parser.read_kind()?;
    let pubkey = parser.read_pubkey()?;

    if kind == 1 && pubkey == target_pubkey {
        // Only deserialize matching events
        let full_event = parser.into_note()?;
        results.push(full_event);
    }
}
```

**Expected improvement:** 10-100x faster for filter-heavy workloads

---

### 3. Batch Serialization with Shared Buffer

For serializing many events (e.g., database writes):

```rust
/// Batch serializer that reuses internal buffers
pub struct NotePacker {
    buf: Vec<u8>,
}

impl NotePacker {
    pub fn new() -> Self {
        Self { buf: Vec::with_capacity(4096) }
    }

    /// Pack an event, returning bytes (buffer is reused internally)
    pub fn pack(&mut self, note: &NoteBinary) -> &[u8] {
        self.buf.clear();
        note.pack_into(&mut self.buf);
        &self.buf
    }

    /// Pack directly into a destination buffer
    pub fn pack_into(&self, note: &NoteBinary, dest: &mut Vec<u8>) {
        note.pack_into(dest);
    }
}
```

**Use case:** Batch database insertion:

```rust
let mut packer = NotePacker::new();
for event in events {
    let bytes = packer.pack(&event.to_note_binary());
    database.insert(&event.id, bytes)?;
}
```

---

### 4. Optional: Direct JSON-to-Notepack Path

For the specific relay use case of receiving JSON and storing binary:

```rust
/// Parse JSON and encode to notepack in a single pass
/// Avoids intermediate struct allocations
pub fn json_to_notepack(json: &[u8]) -> Result<Vec<u8>, Error> {
    // Use simd-json or similar for fast JSON parsing
    // Write directly to notepack format as fields are parsed
}

/// Decode notepack and serialize to JSON in a single pass
pub fn notepack_to_json(data: &[u8]) -> Result<String, Error> {
    // Parse notepack fields
    // Write directly to JSON string
}
```

This is a more advanced optimization for maximum throughput.

---

## Wire Format Consideration

To enable zero-copy field access, the wire format should have **fixed-offset fields** at the start:

```
Current (if variable):
[varint: id_len][id: 32 bytes][varint: pubkey_len][pubkey: 32 bytes]...

Optimized (fixed prefix):
[id: 32 bytes][pubkey: 32 bytes][sig: 64 bytes][created_at: 8 bytes][kind: varint][tags...][content...]
```

With a fixed 136-byte prefix (32+32+64+8), field access is O(1):
- `id` at offset 0
- `pubkey` at offset 32
- `sig` at offset 64
- `created_at` at offset 128

This matches how DannyPack achieves fast field access.

---

## Summary of Changes

| Change | Effort | Impact |
|--------|--------|--------|
| `NoteBinary` struct with references | Low | ~2-3x serialize |
| `pack_into(&mut Vec<u8>)` method | Low | Reduces allocations |
| `read_kind()`, `read_pubkey()` | Medium | 10-100x for filtering |
| Fixed-offset wire format | High | Enables O(1) field access |
| JSON direct path | High | Maximum throughput |

## Benchmark Comparison Target

After optimizations, notepack should achieve:

| Metric | Current | Target | Proto Binary |
|--------|---------|--------|--------------|
| Serialize | 4.08 ms | <1.5 ms | 1.72 ms |
| Deserialize | 2.98 ms | <2.5 ms | 2.04 ms |
| Size | 64.6% | 64.6% | 88.9% |

**Goal:** Match or beat protobuf speed while maintaining notepack's size advantage.

---

## Questions for Discussion

1. Is the wire format already fixed-offset for id/pubkey/sig, or variable-length?
2. Would a breaking wire format change be acceptable for v2?
3. Should `NoteBinary` be the primary API, with `NoteBuf` as a convenience wrapper?
4. Interest in SIMD-accelerated hex encoding for the legacy path?



