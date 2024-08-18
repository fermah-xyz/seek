use bincode::{DefaultOptions, Options};
use serde::{de::DeserializeOwned, Serialize};

use crate::serialization::error::Error;

/// Trait for conversion to and from bincode bytes
pub trait ToBincodeBytes: Serialize {
    fn to_bincode_bytes(&self) -> Result<Vec<u8>, Error>;
}

pub trait FromBincodeBytes: DeserializeOwned {
    fn from_bincode_bytes(bytes: &[u8]) -> Result<Self, Error>
    where
        Self: Sized;
}

/// Default impl for types that derive Serde
impl<T: Serialize> ToBincodeBytes for T {
    fn to_bincode_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(DefaultOptions::new().serialize(&self)?)
    }
}

impl<T: DeserializeOwned> FromBincodeBytes for T {
    fn from_bincode_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(DefaultOptions::new().deserialize(bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct TestStruct {
        b: String,
    }

    #[test]
    fn test_to_bytes() {
        let v = TestStruct {
            b: "test".to_string(),
        };

        let bytes = v.to_bincode_bytes().unwrap();
        let expected = vec![4, 116, 101, 115, 116];
        assert_eq!(bytes, expected);
    }

    #[test]
    fn test_from_bytes() {
        let bytes = vec![4, 116, 101, 115, 116];
        let v = TestStruct::from_bincode_bytes(&bytes).unwrap();
        let expected = TestStruct {
            b: "test".to_string(),
        };
        assert_eq!(v.b, expected.b);
    }
}
