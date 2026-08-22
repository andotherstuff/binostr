//! Shared reversible encoding helpers used by the binary format variants.

/// Decode a lowercase hexadecimal string when doing so is both reversible and
/// worthwhile. Nostr IDs and public keys use lowercase hex, while arbitrary tag
/// strings are case-sensitive and must never be normalized accidentally.
pub(crate) fn decode_lower_hex(value: &str) -> Option<Vec<u8>> {
    let bytes = value.as_bytes();
    if bytes.len() < 8 || !bytes.len().is_multiple_of(2) {
        return None;
    }

    if !bytes
        .iter()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return None;
    }

    hex::decode(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_compacts_reversible_lowercase_hex() {
        assert_eq!(
            decode_lower_hex("deadbeef"),
            Some(vec![0xde, 0xad, 0xbe, 0xef])
        );
        assert_eq!(decode_lower_hex("DEADBEEF"), None);
        assert_eq!(decode_lower_hex("cafe"), None);
        assert_eq!(decode_lower_hex("not hex!"), None);
    }
}
