use std::fmt::{Debug, Display};

use const_hex::{traits::FromHex, FromHexError, ToHexExt};
use ethers::types::H256;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use sled::IVec;

use crate::hash::Hasher;

#[derive(Serialize, Deserialize, Clone, Copy, Hash, Eq, PartialEq)]
pub struct Keccak256Hash(pub H256);
impl AsRef<[u8]> for Keccak256Hash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl From<[u8; 32]> for Keccak256Hash {
    fn from(value: [u8; 32]) -> Self {
        Keccak256Hash(H256::from(value))
    }
}

impl From<IVec> for Keccak256Hash {
    fn from(value: IVec) -> Self {
        Keccak256Hash(H256::from_slice(value.as_ref()))
    }
}

impl Debug for Keccak256Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encode_hex_with_prefix())
    }
}

impl Display for Keccak256Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encode_hex_with_prefix())
    }
}

impl FromHex for Keccak256Hash {
    type Error = FromHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let bytes = <Vec<u8>>::from_hex(hex)?;
        Ok(Keccak256Hash(H256::from_slice(bytes.as_slice())))
    }
}

#[derive(Clone)]
pub struct Keccak256Hasher(Keccak256);

impl Hasher for Keccak256Hasher {
    type Hash = Keccak256Hash;

    fn new() -> Self {
        Self(Keccak256::new())
    }

    fn update(&mut self, data: &[u8]) -> &mut Self {
        self.0.update(data);
        self
    }

    /// Unimplemented
    fn update_mmap_rayon(&mut self, _path: &std::path::Path) -> Result<(), std::io::Error> {
        Ok(())
    }

    /// Unimplemented
    fn update_mmap(&mut self, _path: &std::path::Path) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn finalize(self) -> Self::Hash {
        Keccak256Hash(H256::from_slice(self.0.finalize().as_slice()))
    }
}

#[cfg(test)]
mod tests {
    use const_hex::{traits::FromHex, ToHexExt};

    use crate::hash::{
        keccak256::{Keccak256Hash, Keccak256Hasher},
        Hashable,
    };

    struct TestHashable {
        data: String,
    }

    impl Hashable for TestHashable {
        fn collect(&self) -> std::borrow::Cow<[u8]> {
            self.data.as_bytes().into()
        }
    }

    #[test]
    fn test_hashable() {
        let hex = "0x9c22ff5f21f0b81b113e63f7db6da94fedef11b2119b4088b89664fb9a3cb658";

        let th = TestHashable {
            data: "test".to_string(),
        };

        let hash = th.hash::<Keccak256Hasher>();
        assert_eq!(hash.encode_hex_with_prefix(), hex);

        let hexed = Keccak256Hash::from_hex(hex).unwrap();
        assert_eq!(hash, hexed);
    }
}
