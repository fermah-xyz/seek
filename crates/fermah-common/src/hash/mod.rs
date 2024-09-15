use std::borrow::Cow;

use const_hex::{traits::FromHex, FromHexError};
use ethers::types::Address;

pub mod blake3;
pub mod keccak256;

/// Hasher trait that defines the common interface for hashing algorithms.
pub trait Hasher {
    type Hash: FromHex<Error = FromHexError>;

    fn new() -> Self;
    fn update(&mut self, data: &[u8]) -> &mut Self;
    fn update_mmap_rayon(&mut self, path: &std::path::Path) -> Result<(), std::io::Error>;
    fn update_mmap(&mut self, path: &std::path::Path) -> Result<(), std::io::Error>;
    fn finalize(self) -> Self::Hash;
}

/// Hashable trait for structs that can be hashed.
/// The trait provides a way to collect the data to be hashed.
pub trait Hashable {
    fn collect(&self) -> Cow<[u8]>;

    fn hash<HSH: Hasher>(&self) -> HSH::Hash {
        let mut hasher = HSH::new();
        hasher.update(&self.collect());
        hasher.finalize()
    }
}

impl Hashable for Address {
    fn collect(&self) -> Cow<[u8]> {
        self.as_bytes().into()
    }
}
