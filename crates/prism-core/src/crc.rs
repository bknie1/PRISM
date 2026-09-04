//! CRC32 (IEEE polynomial 0xEDB88320), as used per chunk (docs/container.md).

const fn build_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
}

const TABLE: [u32; 256] = build_table();

/// CRC32 over the concatenation of `parts`.
pub fn crc32_parts(parts: &[&[u8]]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for part in parts {
        for &byte in *part {
            c = TABLE[((c ^ byte as u32) & 0xFF) as usize] ^ (c >> 8);
        }
    }
    c ^ 0xFFFF_FFFF
}

pub fn crc32(data: &[u8]) -> u32 {
    crc32_parts(&[data])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn parts_equal_concatenation() {
        assert_eq!(crc32_parts(&[b"1234", b"56789"]), crc32(b"123456789"));
    }
}
