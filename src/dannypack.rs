//! DannyPack: Ultra-compact custom binary format for Nostr events
//!
//! BLAZINGLY FAST - uses unsafe pointer operations for maximum speed.
//!
//! Layout (single event):
//! ```text
//! [fixed: 138 bytes]
//!   - id: 32 bytes
//!   - pubkey: 32 bytes  
//!   - sig: 64 bytes
//!   - created_at: 8 bytes (i64 LE)
//!   - kind: 2 bytes (u16 LE)
//! [tag_len: varint] + [tag_data: variable]
//! [content_header: 1 byte (bit7=is_hex, bits0-6=len or 0x7F for varint)] + [content_data]
//! ```

use crate::event::NostrEvent;
use std::ptr;

const FIXED_SIZE: usize = 138;

// Lowercase-only hex decode table - rejects A-F to preserve case through roundtrip
const HEX_LUT_LOWER: [u8; 256] = {
    let mut t = [0xFFu8; 256];
    t[b'0' as usize] = 0;
    t[b'1' as usize] = 1;
    t[b'2' as usize] = 2;
    t[b'3' as usize] = 3;
    t[b'4' as usize] = 4;
    t[b'5' as usize] = 5;
    t[b'6' as usize] = 6;
    t[b'7' as usize] = 7;
    t[b'8' as usize] = 8;
    t[b'9' as usize] = 9;
    t[b'a' as usize] = 10;
    t[b'b' as usize] = 11;
    t[b'c' as usize] = 12;
    t[b'd' as usize] = 13;
    t[b'e' as usize] = 14;
    t[b'f' as usize] = 15;
    // Note: A-F intentionally NOT mapped - we reject uppercase to preserve case
    t
};

const HEX_PAIR_LUT: [u16; 256] = {
    let mut t = [0u16; 256];
    let chars = b"0123456789abcdef";
    let mut i = 0;
    while i < 256 {
        let hi = chars[i >> 4];
        let lo = chars[i & 0xF];
        // We want memory to be [hi, lo].
        // On LE machine, u16 = hi | (lo << 8).
        // On BE machine, u16 = (hi << 8) | lo.
        // We assume LE for simplicity or use to_le_bytes logic if we write as bytes.
        // But ptr::write_unaligned takes value.
        // Let's construct LE value and use to_le if needed, or just construct for LE.
        // Most targets are LE.
        t[i] = (hi as u16) | ((lo as u16) << 8);
        i += 1;
    }
    t
};

#[inline(always)]
unsafe fn hex_encode_fast(src: &[u8], dst: *mut u8) -> usize {
    let len = src.len();
    let mut i = 0;
    while i < len {
        let b = *src.get_unchecked(i);
        let pair = *HEX_PAIR_LUT.get_unchecked(b as usize);
        ptr::write_unaligned(dst.add(i * 2) as *mut u16, pair);
        i += 1;
    }
    len * 2
}

#[inline(always)]
unsafe fn write_varint_ptr(mut dst: *mut u8, mut value: u64) -> usize {
    let start = dst;
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        *dst = if value != 0 { byte | 0x80 } else { byte };
        dst = dst.add(1);
        if value == 0 {
            break;
        }
    }
    dst.offset_from(start) as usize
}

#[inline(always)]
unsafe fn read_varint_ptr(src: *const u8, max_len: usize) -> (u64, usize) {
    let mut result: u64 = 0;
    let mut shift = 0;
    let mut pos = 0;
    loop {
        if pos >= max_len {
            return (0, 0);
        }
        let byte = *src.add(pos);
        pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    (result, pos)
}

#[inline(always)]
unsafe fn write_len_flag_ptr(dst: *mut u8, len: usize, is_hex: bool) -> usize {
    let flag = if is_hex { 0x80u8 } else { 0x00u8 };
    if len < 0x7F {
        *dst = flag | (len as u8);
        1
    } else {
        *dst = flag | 0x7F;
        1 + write_varint_ptr(dst.add(1), len as u64)
    }
}

#[inline(always)]
unsafe fn read_len_flag_ptr(src: *const u8, max_len: usize) -> (usize, bool, usize) {
    let header = *src;
    let is_hex = (header & 0x80) != 0;
    let len_or_marker = (header & 0x7F) as usize;
    if len_or_marker < 0x7F {
        (len_or_marker, is_hex, 1)
    } else {
        let (len, varint_bytes) = read_varint_ptr(src.add(1), max_len - 1);
        (len as usize, is_hex, 1 + varint_bytes)
    }
}

/// is this POSSIBLY hex?
#[inline(always)]
unsafe fn might_be_hex(src: &[u8]) -> bool {
    let len = src.len();
    // Must be even, >= 8 chars (4 bytes min for worthwhile compression)
    // AND first 8 chars must be hex (reject text early)
    // We only support lowercase hex for compression to preserve case
    if len < 8 || len & 1 != 0 {
        return false;
    }

    // Check first 8 bytes (IDs, keys and pubs are all divisible by 8)
    // Unrolling helps
    let p = src.as_ptr();
    let mask = (*HEX_LUT_LOWER.get_unchecked(*p as usize))
        | (*HEX_LUT_LOWER.get_unchecked(*p.add(1) as usize))
        | (*HEX_LUT_LOWER.get_unchecked(*p.add(2) as usize))
        | (*HEX_LUT_LOWER.get_unchecked(*p.add(3) as usize))
        | (*HEX_LUT_LOWER.get_unchecked(*p.add(4) as usize))
        | (*HEX_LUT_LOWER.get_unchecked(*p.add(5) as usize))
        | (*HEX_LUT_LOWER.get_unchecked(*p.add(6) as usize))
        | (*HEX_LUT_LOWER.get_unchecked(*p.add(7) as usize));

    mask != 0xFF
}

/// Hex decode - assumes caller already checked might_be_hex()
/// Returns decoded length, or 0 if invalid/uppercase hex encountered
/// Only accepts lowercase hex (0-9, a-f) to preserve case through roundtrip
#[inline(always)]
unsafe fn hex_decode_checked(src: &[u8], dst: *mut u8) -> usize {
    let len = src.len();
    let out_len = len >> 1;
    let mut i = 0;
    while i + 8 <= len {
        let h0 = *HEX_LUT_LOWER.get_unchecked(*src.get_unchecked(i) as usize);
        let l0 = *HEX_LUT_LOWER.get_unchecked(*src.get_unchecked(i + 1) as usize);
        let h1 = *HEX_LUT_LOWER.get_unchecked(*src.get_unchecked(i + 2) as usize);
        let l1 = *HEX_LUT_LOWER.get_unchecked(*src.get_unchecked(i + 3) as usize);
        let h2 = *HEX_LUT_LOWER.get_unchecked(*src.get_unchecked(i + 4) as usize);
        let l2 = *HEX_LUT_LOWER.get_unchecked(*src.get_unchecked(i + 5) as usize);
        let h3 = *HEX_LUT_LOWER.get_unchecked(*src.get_unchecked(i + 6) as usize);
        let l3 = *HEX_LUT_LOWER.get_unchecked(*src.get_unchecked(i + 7) as usize);

        if (h0 | l0 | h1 | l1 | h2 | l2 | h3 | l3) & 0xF0 != 0 {
            return 0;
        }

        let out_idx = i >> 1;
        *dst.add(out_idx) = (h0 << 4) | l0;
        *dst.add(out_idx + 1) = (h1 << 4) | l1;
        *dst.add(out_idx + 2) = (h2 << 4) | l2;
        *dst.add(out_idx + 3) = (h3 << 4) | l3;
        i += 8;
    }

    while i + 2 <= len {
        let hi = *HEX_LUT_LOWER.get_unchecked(*src.get_unchecked(i) as usize);
        let lo = *HEX_LUT_LOWER.get_unchecked(*src.get_unchecked(i + 1) as usize);
        if (hi | lo) & 0xF0 != 0 {
            return 0;
        }
        *dst.add(i >> 1) = (hi << 4) | lo;
        i += 2;
    }

    out_len
}

#[inline(always)]
const fn varint_size(mut value: u64) -> usize {
    let mut size = 1;
    while value >= 0x80 {
        value >>= 7;
        size += 1;
    }
    size
}

pub fn serialize(event: &NostrEvent, buf: &mut Vec<u8>) {
    let max_tags_size = calc_max_tags_size(&event.tags);
    let content_len = event.content.len();
    let estimated = FIXED_SIZE + 5 + max_tags_size + 5 + content_len;

    buf.reserve(estimated);

    unsafe {
        let original_len = buf.len();
        let base = buf.as_mut_ptr().add(original_len);
        let mut ptr = base;

        ptr::copy_nonoverlapping(event.id.as_ptr(), ptr, 32);
        ptr::copy_nonoverlapping(event.pubkey.as_ptr(), ptr.add(32), 32);
        ptr::copy_nonoverlapping(event.sig.as_ptr(), ptr.add(64), 64);
        ptr = ptr.add(128);

        *(ptr as *mut [u8; 8]) = event.created_at.to_le_bytes();
        ptr = ptr.add(8);
        *(ptr as *mut [u8; 2]) = event.kind.to_le_bytes();
        ptr = ptr.add(2);

        let tag_len_ptr = ptr;
        ptr = ptr.add(5);
        let tag_data_start = ptr;

        ptr = pack_tags_fast(ptr, &event.tags);

        let tag_data_len = ptr.offset_from(tag_data_start) as usize;
        let varint_len = write_varint_ptr(tag_len_ptr, tag_data_len as u64);
        if varint_len < 5 {
            ptr::copy(tag_data_start, tag_len_ptr.add(varint_len), tag_data_len);
            ptr = tag_len_ptr.add(varint_len + tag_data_len);
        }

        let content_bytes = event.content.as_bytes();

        if might_be_hex(content_bytes) {
            let header_ptr = ptr;
            ptr = ptr.add(5);
            let decoded_len = hex_decode_checked(content_bytes, ptr);
            if decoded_len > 0 {
                let header_len = write_len_flag_ptr(header_ptr, decoded_len, true);
                if header_len < 5 {
                    ptr::copy(ptr, header_ptr.add(header_len), decoded_len);
                }
                ptr = header_ptr.add(header_len + decoded_len);
            } else {
                ptr = header_ptr;
                let len = content_bytes.len();
                let header_len = write_len_flag_ptr(ptr, len, false);
                ptr = ptr.add(header_len);
                ptr::copy_nonoverlapping(content_bytes.as_ptr(), ptr, len);
                ptr = ptr.add(len);
            }
        } else {
            let len = content_bytes.len();
            let header_len = write_len_flag_ptr(ptr, len, false);
            ptr = ptr.add(header_len);
            ptr::copy_nonoverlapping(content_bytes.as_ptr(), ptr, len);
            ptr = ptr.add(len);
        }

        let written = ptr.offset_from(base) as usize;
        buf.set_len(original_len + written);
    }
}

#[inline(always)]
fn calc_max_tags_size(tags: &[Vec<String>]) -> usize {
    let mut size = varint_size(tags.len() as u64);
    for tag in tags {
        size += 1;
        for value in tag {
            let len = value.len();
            size += if len < 0x7F {
                1
            } else {
                1 + varint_size(len as u64)
            };
            size += len; // worst case: no hex compression
        }
    }
    size
}

#[inline(always)]
unsafe fn pack_tags_fast(mut dst: *mut u8, tags: &[Vec<String>]) -> *mut u8 {
    dst = dst.add(write_varint_ptr(dst, tags.len() as u64));

    for tag in tags {
        *dst = tag.len() as u8;
        dst = dst.add(1);

        for value in tag {
            let bytes = value.as_bytes();
            let len = bytes.len();

            if might_be_hex(bytes) {
                let header_ptr = dst;
                dst = dst.add(5);
                let decoded_len = hex_decode_checked(bytes, dst);
                if decoded_len > 0 {
                    let header_len = write_len_flag_ptr(header_ptr, decoded_len, true);
                    if header_len < 5 {
                        ptr::copy(dst, header_ptr.add(header_len), decoded_len);
                    }
                    dst = header_ptr.add(header_len + decoded_len);
                } else {
                    dst = header_ptr;
                    let header_len = write_len_flag_ptr(dst, len, false);
                    dst = dst.add(header_len);
                    ptr::copy_nonoverlapping(bytes.as_ptr(), dst, len);
                    dst = dst.add(len);
                }
            } else {
                let header_len = write_len_flag_ptr(dst, len, false);
                dst = dst.add(header_len);
                ptr::copy_nonoverlapping(bytes.as_ptr(), dst, len);
                dst = dst.add(len);
            }
        }
    }

    dst
}

pub fn deserialize(data: &[u8]) -> Result<NostrEvent, DannyPackError> {
    let mut event = NostrEvent {
        id: [0u8; 32],
        pubkey: [0u8; 32],
        created_at: 0,
        kind: 0,
        tags: Vec::new(),
        content: String::new(),
        sig: [0u8; 64],
    };
    deserialize_into(data, &mut event)?;
    Ok(event)
}

pub fn deserialize_into(data: &[u8], event: &mut NostrEvent) -> Result<(), DannyPackError> {
    let len = data.len();
    if len < FIXED_SIZE + 2 {
        return Err(DannyPackError::TooShort);
    }

    unsafe {
        let base = data.as_ptr();
        let mut ptr = base;

        ptr::copy_nonoverlapping(ptr, event.id.as_mut_ptr(), 32);
        ptr = ptr.add(32);
        ptr::copy_nonoverlapping(ptr, event.pubkey.as_mut_ptr(), 32);
        ptr = ptr.add(32);
        ptr::copy_nonoverlapping(ptr, event.sig.as_mut_ptr(), 64);
        ptr = ptr.add(64);

        event.created_at = i64::from_le_bytes(*(ptr as *const [u8; 8]));
        ptr = ptr.add(8);

        event.kind = u16::from_le_bytes(*(ptr as *const [u8; 2]));
        ptr = ptr.add(2);

        let remaining = len - (ptr.offset_from(base) as usize);

        let (tag_len, varint_bytes) = read_varint_ptr(ptr, remaining);
        if varint_bytes == 0 {
            return Err(DannyPackError::TooShort);
        }
        ptr = ptr.add(varint_bytes);
        let tag_len = tag_len as usize;

        let remaining = len - (ptr.offset_from(base) as usize);
        if tag_len > remaining {
            return Err(DannyPackError::TooShort);
        }

        unpack_tags_into(ptr, tag_len, &mut event.tags)?;
        ptr = ptr.add(tag_len);

        let remaining = len - (ptr.offset_from(base) as usize);
        let (content_len, content_is_hex, header_bytes) = read_len_flag_ptr(ptr, remaining);
        ptr = ptr.add(header_bytes);

        let remaining = len - (ptr.offset_from(base) as usize);
        if content_len > remaining {
            return Err(DannyPackError::TooShort);
        }

        if content_is_hex {
            let required = content_len * 2;
            event.content.clear();
            event.content.reserve(required);
            let vec = event.content.as_mut_vec();
            vec.set_len(required);
            hex_encode_fast(
                std::slice::from_raw_parts(ptr, content_len),
                vec.as_mut_ptr(),
            );
        } else {
            event.content.clear();
            event.content.reserve(content_len);
            let vec = event.content.as_mut_vec();
            ptr::copy_nonoverlapping(ptr, vec.as_mut_ptr(), content_len);
            vec.set_len(content_len);
        }
    }

    Ok(())
}

#[inline(always)]
unsafe fn unpack_tags_into(
    ptr: *const u8,
    max_len: usize,
    tags: &mut Vec<Vec<String>>,
) -> Result<(), DannyPackError> {
    if max_len == 0 {
        tags.clear();
        return Ok(());
    }

    let mut pos = 0;

    let (tag_count, varint_bytes) = read_varint_ptr(ptr, max_len);
    if varint_bytes == 0 {
        return Err(DannyPackError::InvalidTagData);
    }
    pos += varint_bytes;
    let tag_count = tag_count as usize;

    if tags.capacity() < tag_count {
        tags.reserve(tag_count - tags.len());
    }
    // We don't clear tags yet, we overwrite them.
    // But since tags is Vec<Vec<String>>, we want to reuse the inner Vecs.

    for i in 0..tag_count {
        if pos >= max_len {
            return Err(DannyPackError::InvalidTagData);
        }

        let value_count = *ptr.add(pos) as usize;
        pos += 1;

        if i >= tags.len() {
            tags.push(Vec::with_capacity(value_count));
        }
        let values = tags.get_unchecked_mut(i);
        // We reuse values vec, but clearing it drops strings.
        // We want to reuse strings too!

        // Resize values vec to match value_count, creating new strings if needed
        if values.len() < value_count {
            values.resize(value_count, String::new());
        }
        for j in 0..value_count {
            let remaining = max_len - pos;
            let (len, is_hex, header_bytes) = read_len_flag_ptr(ptr.add(pos), remaining);
            pos += header_bytes;

            if pos + len > max_len {
                return Err(DannyPackError::InvalidTagData);
            }

            let s = values.get_unchecked_mut(j);

            if is_hex {
                let required = len * 2;
                s.clear();
                s.reserve(required);
                let vec = s.as_mut_vec();
                vec.set_len(required);
                hex_encode_fast(
                    std::slice::from_raw_parts(ptr.add(pos), len),
                    vec.as_mut_ptr(),
                );
            } else {
                s.clear();
                s.reserve(len);
                let vec = s.as_mut_vec();
                ptr::copy_nonoverlapping(ptr.add(pos), vec.as_mut_ptr(), len);
                vec.set_len(len);
            }

            pos += len;
        }
        values.truncate(value_count);
    }
    tags.truncate(tag_count);

    Ok(())
}

pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(events.len() * 200 + 4);
    buf.extend_from_slice(&(events.len() as u32).to_le_bytes());

    for event in events {
        let len_pos = buf.len();
        buf.extend_from_slice(&[0u8; 4]);

        serialize(event, &mut buf);

        let event_len = buf.len() - len_pos - 4;
        let len_bytes = (event_len as u32).to_le_bytes();
        // Write length back
        buf[len_pos..len_pos + 4].copy_from_slice(&len_bytes);
    }

    buf
}

pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, DannyPackError> {
    let len = data.len();
    if len < 4 {
        return Err(DannyPackError::TooShort);
    }

    unsafe {
        let base = data.as_ptr();
        let mut ptr = base;

        let event_count = u32::from_le_bytes(*(ptr as *const [u8; 4])) as usize;
        ptr = ptr.add(4);

        let mut events = Vec::with_capacity(event_count);

        for _ in 0..event_count {
            let remaining = len - (ptr.offset_from(base) as usize);
            if remaining < 4 {
                return Err(DannyPackError::TooShort);
            }

            let event_len = u32::from_le_bytes(*(ptr as *const [u8; 4])) as usize;
            ptr = ptr.add(4);

            let remaining = len - (ptr.offset_from(base) as usize);
            if event_len > remaining {
                return Err(DannyPackError::TooShort);
            }

            events.push(deserialize(std::slice::from_raw_parts(ptr, event_len))?);
            ptr = ptr.add(event_len);
        }

        Ok(events)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DannyPackError {
    #[error("Data too short")]
    TooShort,

    #[error("Invalid tag data")]
    InvalidTagData,

    #[error("Invalid varint")]
    InvalidVarint,

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> NostrEvent {
        NostrEvent {
            id: [0xab; 32],
            pubkey: [0xcd; 32],
            created_at: 1234567890,
            kind: 1,
            tags: vec![
                vec!["p".to_string(), "abcd1234".to_string()],
                vec!["e".to_string(), "deadbeef".to_string()],
            ],
            content: "Hello, Nostr!".to_string(),
            sig: [0xef; 64],
        }
    }

    fn sample_event_hex_content() -> NostrEvent {
        NostrEvent {
            id: [0xab; 32],
            pubkey: [0xcd; 32],
            created_at: 1234567890,
            kind: 1,
            tags: vec![],
            content: "deadbeefcafe1234".to_string(),
            sig: [0xef; 64],
        }
    }

    #[test]
    fn test_roundtrip() {
        let event = sample_event();
        let mut bytes = Vec::new();
        serialize(&event, &mut bytes);
        let back = deserialize(&bytes).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_roundtrip_hex_content() {
        let event = sample_event_hex_content();
        let mut bytes = Vec::new();
        serialize(&event, &mut bytes);
        let back = deserialize(&bytes).unwrap();
        assert_eq!(event, back);

        let non_hex = sample_event();
        let mut non_hex_bytes = Vec::new();
        serialize(&non_hex, &mut non_hex_bytes);
        println!("Normal content: {} bytes", non_hex_bytes.len());
        println!("Hex content:    {} bytes", bytes.len());
    }

    #[test]
    fn test_batch_roundtrip() {
        let events = vec![sample_event(), sample_event_hex_content()];
        let bytes = serialize_batch(&events);
        let back = deserialize_batch(&bytes).unwrap();
        assert_eq!(events, back);
    }

    #[test]
    fn test_size_comparison() {
        let event = sample_event();

        let mut dannypack_buf = Vec::new();
        serialize(&event, &mut dannypack_buf);
        let dannypack_size = dannypack_buf.len();
        let json_size = crate::json::serialize(&event).len();

        println!("DannyPack: {} bytes", dannypack_size);
        println!("JSON:      {} bytes", json_size);
        println!(
            "Savings:   {:.1}%",
            100.0 * (1.0 - dannypack_size as f64 / json_size as f64)
        );
    }
}
