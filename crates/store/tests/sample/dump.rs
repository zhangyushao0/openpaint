//! The record dump format of the frozen samples (see `tests/schema.rs`).

use super::Records;
use crate::common::KEYSPACES;

pub fn encode(records: &Records) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (keyspace, key, value) in records {
        let index = KEYSPACES
            .iter()
            .position(|name| name == keyspace)
            .expect("known");
        bytes.push(u8::try_from(index).expect("small"));
        for part in [key, value] {
            bytes.extend_from_slice(&u32::try_from(part.len()).expect("len").to_le_bytes());
            bytes.extend_from_slice(part);
        }
    }
    bytes
}

fn take<'a>(bytes: &'a [u8], at: &mut usize, len: usize) -> &'a [u8] {
    let part = &bytes[*at..*at + len];
    *at += len;
    part
}

pub fn decode(bytes: &[u8]) -> Records {
    let mut at = 0;
    let mut out = Vec::new();
    while at < bytes.len() {
        let index = take(bytes, &mut at, 1)[0];
        let mut part = || {
            let len = take(bytes, &mut at, 4).try_into().expect("length");
            take(bytes, &mut at, u32::from_le_bytes(len) as usize).to_vec()
        };
        let (key, value) = (part(), part());
        out.push((KEYSPACES[usize::from(index)].to_owned(), key, value));
    }
    out
}
