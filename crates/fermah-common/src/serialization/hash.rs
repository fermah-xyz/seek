use std::borrow::Cow;

use const_hex::{traits::FromHex, FromHexError, ToHexExt};
use serde::{Deserialize, Serialize};

use super::encoding::hex_encoded;
use crate::hash::{Hashable, Hasher};

/// Serializable struct type for a generic hash.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializableHash<HSH: Hasher>(#[serde(with = "hex_encoded")] pub HSH::Hash)
where
    HSH::Hash: ToHexExt + FromHex;

impl<HSH: Hasher> FromHex for SerializableHash<HSH>
where
    HSH::Hash: ToHexExt + FromHex,
{
    type Error = FromHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let h = HSH::Hash::from_hex(hex.as_ref())?;
        Ok(Self(h))
    }
}

impl<HSH: Hasher> Hashable for SerializableHash<HSH>
where
    HSH::Hash: ToHexExt + FromHex + AsRef<[u8]>,
{
    fn collect(&self) -> Cow<[u8]> {
        self.0.as_ref().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::blake3::Blake3Hasher;

    struct TestData {
        data: Vec<u8>,
    }

    impl Hashable for TestData {
        fn collect(&self) -> Cow<[u8]> {
            self.data.as_slice().into()
        }
    }

    #[test]
    fn test_serializable_hash() {
        let hash = TestData { data: vec![1; 32] }.hash::<Blake3Hasher>();
        let serializable_hash = SerializableHash::<Blake3Hasher>(hash);

        let serialized = serde_json::to_string(&serializable_hash).unwrap();
        let deserialized: SerializableHash<Blake3Hasher> =
            serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.0, hash);
    }
}
