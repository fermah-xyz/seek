use std::fmt::{Debug, Display};

use const_hex::{traits::FromHex, FromHexError, ToHexExt};
use ethers::abi::FixedBytes;
use serde::{Deserialize, Serialize};

use crate::hash::Hasher;

#[derive(Serialize, Deserialize, Clone, Copy, Hash, Eq, PartialEq)]
pub struct Blake3Hash(pub blake3::Hash);

impl Blake3Hash {
    pub fn as_32_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

impl AsRef<[u8]> for Blake3Hash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl From<[u8; 32]> for Blake3Hash {
    fn from(value: [u8; 32]) -> Self {
        let h: blake3::Hash = value.into();
        Blake3Hash(h)
    }
}

impl From<FixedBytes> for Blake3Hash {
    fn from(value: FixedBytes) -> Self {
        assert_eq!(value.len(), 32, "Length of the array is not equal to 32");
        let mut buf = [0_u8; 32];

        for (i, b) in value.into_iter().enumerate() {
            buf[i] = b;
        }
        buf.into()
    }
}

impl Debug for Blake3Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encode_hex_with_prefix())
    }
}

impl Display for Blake3Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encode_hex_with_prefix())
    }
}

impl FromHex for Blake3Hash {
    type Error = FromHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let bytes = <Vec<u8>>::from_hex(hex)?;
        let fixed: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| FromHexError::InvalidStringLength)?;
        Ok(Blake3Hash(fixed.into()))
    }
}

#[derive(Clone)]
pub struct Blake3Hasher(pub blake3::Hasher);

impl Hasher for Blake3Hasher {
    type Hash = Blake3Hash;

    fn new() -> Self {
        Self(blake3::Hasher::new())
    }

    fn update(&mut self, data: &[u8]) -> &mut Self {
        self.0.update(data);
        self
    }

    fn update_mmap_rayon(&mut self, path: &std::path::Path) -> Result<(), std::io::Error> {
        self.0.update_mmap_rayon(path)?;
        Ok(())
    }

    fn update_mmap(&mut self, path: &std::path::Path) -> Result<(), std::io::Error> {
        self.0.update_mmap(path)?;
        Ok(())
    }

    fn finalize(self) -> Self::Hash {
        Blake3Hash(self.0.finalize())
    }
}

#[cfg(test)]
mod tests {
    use crate::hash::{blake3::Blake3Hasher, Hashable};

    struct TestHashable {
        data: String,
    }

    impl Hashable for TestHashable {
        fn collect(&self) -> std::borrow::Cow<[u8]> {
            self.data.as_bytes().into()
        }
    }

    #[test]
    fn test_hasher() {
        let hex = "4878ca0425c739fa427f7eda20fe845f6b2e46ba5fe2a14df5b1e32f50603215";

        let th = TestHashable {
            data: "test".to_string(),
        };

        let hash = th.hash::<Blake3Hasher>().0;
        assert_eq!(hash.to_hex().as_str(), hex);

        let hexed = blake3::Hash::from_hex(hex).unwrap();
        assert_eq!(hash, hexed);
    }
}
