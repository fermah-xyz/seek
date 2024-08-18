pub mod bytes;
pub mod encoding;
pub mod error;
pub mod hash;
pub mod serializable_error;

#[cfg(test)]
mod tests {
    use blake3::Hash;
    use encoding::{base64_encoded, hex_encoded};
    use ethers::types::H256;
    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::{
        hash::{blake3::Blake3Hash, keccak256::Keccak256Hash},
        operator::OperatorId,
    };

    #[derive(Serialize, Deserialize, Debug)]
    struct TestStruct {
        #[serde(with = "hex_encoded")]
        hash_blake: Blake3Hash,
        #[serde(with = "hex_encoded")]
        hex: Vec<u8>,
        #[serde(with = "hex_encoded")]
        opid: OperatorId,
        #[serde(with = "base64_encoded")]
        base64: Vec<u8>,
    }

    #[test]
    fn test_encodings() {
        let test = TestStruct {
            hash_blake: Blake3Hash(Hash::from([1; 32])),
            hex: vec![1; 32],
            // hex_32: [1; 32],
            opid: OperatorId::from(Keccak256Hash(H256::from_slice(vec![1; 32].as_slice()))),
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
