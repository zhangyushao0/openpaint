//! Byte-level encoding of stored values: fixed-width little-endian integers and floats,
//! length-prefixed UTF-8 strings. Reading checks every length against the bytes left, so a
//! corrupt record is an error, never a panic or a huge allocation.

use openpaint_app::PortError;

/// Appends values to a record.
#[derive(Debug, Default)]
pub(crate) struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(crate) fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn f32(&mut self, value: f32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// A count or length, which the store keeps below 2^32.
    pub(crate) fn len(&mut self, len: usize) {
        self.u32(u32::try_from(len).unwrap_or(u32::MAX));
    }

    pub(crate) fn str(&mut self, text: &str) {
        self.len(text.len());
        self.bytes.extend_from_slice(text.as_bytes());
    }

    /// Raw bytes without a length: only as the last field of a record.
    pub(crate) fn bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

/// Reads values from a record; every error names the record.
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    what: &'static str,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8], what: &'static str) -> Self {
        Self { bytes, what }
    }

    pub(crate) fn corrupt(&self) -> PortError {
        PortError::Corrupt { what: self.what }
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], PortError> {
        let (head, rest) = self.bytes.split_first_chunk().ok_or(self.corrupt())?;
        self.bytes = rest;
        Ok(*head)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, PortError> {
        Ok(self.take::<1>()?[0])
    }

    pub(crate) fn bool(&mut self) -> Result<bool, PortError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(self.corrupt()),
        }
    }

    pub(crate) fn u32(&mut self) -> Result<u32, PortError> {
        self.take().map(u32::from_le_bytes)
    }

    pub(crate) fn u64(&mut self) -> Result<u64, PortError> {
        self.take().map(u64::from_le_bytes)
    }

    pub(crate) fn f32(&mut self) -> Result<f32, PortError> {
        self.take().map(f32::from_le_bytes)
    }

    /// A count of items of at least `item_bytes` each; refused when the bytes left cannot hold
    /// that many, so a corrupt count never allocates.
    pub(crate) fn count(&mut self, item_bytes: usize) -> Result<usize, PortError> {
        let count = usize::try_from(self.u32()?).map_err(|_| self.corrupt())?;
        let needed = count.checked_mul(item_bytes).ok_or(self.corrupt())?;
        if needed > self.bytes.len() {
            return Err(self.corrupt());
        }
        Ok(count)
    }

    pub(crate) fn str(&mut self) -> Result<String, PortError> {
        let len = self.count(1)?;
        let (text, rest) = self.bytes.split_at(len);
        self.bytes = rest;
        String::from_utf8(text.to_vec()).map_err(|_| self.corrupt())
    }

    /// Every byte not read yet: the last field of a record.
    pub(crate) fn rest(&mut self) -> &'a [u8] {
        std::mem::take(&mut self.bytes)
    }

    /// Succeeds only when every byte was read.
    pub(crate) fn end(&self) -> Result<(), PortError> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(self.corrupt())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip_and_short_input_is_corrupt() {
        let mut writer = Writer::default();
        writer.u8(7);
        writer.bool(true);
        writer.u32(0x0102_0304);
        writer.u64(u64::MAX - 1);
        writer.f32(-0.0);
        writer.str("Ebene ü");
        let bytes = writer.finish();
        let mut reader = Reader::new(&bytes, "test");
        assert_eq!(reader.u8(), Ok(7));
        assert_eq!(reader.bool(), Ok(true));
        assert_eq!(reader.u32(), Ok(0x0102_0304));
        assert_eq!(reader.u64(), Ok(u64::MAX - 1));
        assert_eq!(reader.f32().map(f32::to_bits), Ok((-0.0f32).to_bits()));
        assert_eq!(reader.str().as_deref(), Ok("Ebene ü"));
        assert_eq!(reader.end(), Ok(()));
        let mut short = Reader::new(&bytes[..3], "test");
        assert_eq!(short.u8(), Ok(7));
        assert_eq!(short.u32(), Err(PortError::Corrupt { what: "test" }));
    }

    #[test]
    fn counts_beyond_the_bytes_left_are_corrupt() {
        let mut writer = Writer::default();
        writer.u32(1000);
        writer.u8(0);
        let bytes = writer.finish();
        assert!(Reader::new(&bytes, "count").count(24).is_err());
        assert!(Reader::new(&bytes, "bool").bool().is_err());
        assert!(Reader::new(&[2], "bool").bool().is_err());
    }
}
