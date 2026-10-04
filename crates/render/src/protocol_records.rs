//! Records as GPU bytes: every record that crosses to the GPU writes its fields in declaration
//! order, little endian, which reproduces its `repr(C)` layout (`protocol_tests.rs`).

use super::{Op, Params, TileRecord};

/// A record that crosses to the GPU. Every field is a 4-byte scalar or an array of them, so
/// writing the fields in declaration order reproduces the `repr(C)` layout without padding.
pub(crate) trait Record {
    fn write(&self, out: &mut Vec<u8>);
}

impl Record for Op {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend(self.code.to_le_bytes());
        out.extend(self.data.iter().flat_map(|value| value.to_le_bytes()));
    }
}

impl Record for TileRecord {
    fn write(&self, out: &mut Vec<u8>) {
        let words = [self.origin[0], self.origin[1], self.start, self.count];
        out.extend(words.iter().flat_map(|word| word.to_le_bytes()));
    }
}

impl Record for Params {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend(self.background.iter().flat_map(|value| value.to_le_bytes()));
        let words = [self.tiles, self.row, self.size[0], self.size[1]];
        out.extend(words.iter().flat_map(|word| word.to_le_bytes()));
        out.extend(self.origin.iter().flat_map(|word| word.to_le_bytes()));
        out.extend(self.scale.to_le_bytes());
        out.extend(self.inverse.to_le_bytes());
    }
}

impl Record for u32 {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend(self.to_le_bytes());
    }
}

/// The records as GPU bytes, little endian.
pub(crate) fn encode<R: Record>(records: &[R]) -> Vec<u8> {
    let mut out = Vec::with_capacity(size_of_val(records));
    for record in records {
        record.write(&mut out);
    }
    out
}
