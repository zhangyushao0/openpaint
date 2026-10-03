//! `RgbaImage`: pixels read back from the GPU into memory.

/// 8-bit RGBA pixels, rows tightly packed, top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    size: [u32; 2],
    pixels: Vec<u8>,
}

impl RgbaImage {
    /// `pixels` holds exactly `width * height * 4` bytes; the readback path guarantees it.
    pub(crate) fn new(size: [u32; 2], pixels: Vec<u8>) -> Self {
        debug_assert_eq!(
            pixels.len() as u64,
            u64::from(size[0]) * u64::from(size[1]) * 4
        );
        Self { size, pixels }
    }

    /// `[width, height]` in pixels.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    #[must_use]
    pub fn into_pixels(self) -> Vec<u8> {
        self.pixels
    }

    /// FNV-1a 64 over the size and the pixels: a stable fingerprint for golden tests and
    /// harness output. Not cryptographic.
    pub fn content_hash(&self) -> u64 {
        let size = self.size.iter().flat_map(|side| side.to_le_bytes());
        fnv1a(size.chain(self.pixels.iter().copied()))
    }
}

fn fnv1a(bytes: impl Iterator<Item = u8>) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    bytes.fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_matches_reference_vectors() {
        assert_eq!(fnv1a(std::iter::empty()), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a".iter().copied()), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn hash_depends_on_size_not_only_bytes() {
        let wide = RgbaImage::new([2, 1], vec![0; 8]);
        let tall = RgbaImage::new([1, 2], vec![0; 8]);
        assert_ne!(wide.content_hash(), tall.content_hash());
    }
}
