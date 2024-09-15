#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("hex error: {0}")]
    Hex(#[from] const_hex::FromHexError),
    #[error("utf8 char error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

pub mod hex_encoded {
    use std::fmt::Display;

    use const_hex::{traits::FromHex, ToHexExt};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(x: &T, s: S) -> Result<S::Ok, S::Error>
    where
        T: ToHexExt,
        S: Serializer,
    {
        s.serialize_str(x.encode_hex_with_prefix().as_str())
    }

    pub fn deserialize<'de, T, D>(d: D) -> Result<T, D::Error>
    where
        T: FromHex,
        <T as FromHex>::Error: Display,
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(d)?;
        T::from_hex(buf).map_err(serde::de::Error::custom)
    }
}

pub mod hex_encoded_no_prefix {
    use std::fmt::Display;

    use const_hex::{traits::FromHex, ToHexExt};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(x: &T, s: S) -> Result<S::Ok, S::Error>
    where
        T: ToHexExt,
        S: Serializer,
    {
        s.serialize_str(x.encode_hex().as_str())
    }

    pub fn deserialize<'de, T, D>(d: D) -> Result<T, D::Error>
    where
        T: FromHex,
        <T as FromHex>::Error: Display,
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(d)?;
        T::from_hex(buf).map_err(serde::de::Error::custom)
    }
}

pub mod base64_encoded {
    use base64::{engine::general_purpose, Engine};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let base64 = general_purpose::STANDARD.encode(v);
        String::serialize(&base64, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let base64 = String::deserialize(d)?;
        general_purpose::STANDARD
            .decode(base64.as_bytes())
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use blake3::Hash;
    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::{hash::blake3::Blake3Hash, operator::OperatorId};

    #[derive(Serialize, Deserialize, Debug)]
    struct TestStruct {
        #[serde(with = "hex_encoded")]
        hash_blake: Blake3Hash,
        #[serde(with = "hex_encoded")]
        hex: Vec<u8>,
        opid: OperatorId,
        #[serde(with = "base64_encoded")]
        base64: Vec<u8>,
    }

    #[test]
    fn test_encodings() {
        let test = TestStruct {
            hash_blake: Blake3Hash(Hash::from([1; 32])),
            hex: vec![1; 32],
            opid: OperatorId::from(&[1; 20]),
            base64: vec![1; 32],
        };

        let s = serde_json::to_string_pretty(&test).unwrap();

        println!("{}", s);

        let rs: TestStruct = serde_json::from_str(&s).unwrap();
        assert_eq!(test.hash_blake, rs.hash_blake);
        assert_eq!(test.hex, rs.hex);
        assert_eq!(test.base64, rs.base64);
        println!("{:?}", rs);
    }
}
