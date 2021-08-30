use crate::sm::sm3_hash;

pub const ADDR_BYTES_LEN: usize = 20;
pub const HASH_BYTES_LEN: usize = 32;

pub fn pk2address(pk: &[u8]) -> Vec<u8> {
    sm3_hash(pk)[HASH_BYTES_LEN - ADDR_BYTES_LEN..].to_vec()
}
