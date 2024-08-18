use std::{
    borrow::Cow,
    fmt::{Debug, Display},
};

use const_hex::ToHexExt;
use serde::{Deserialize, Serialize};

use crate::{
    hash::{blake3::Blake3Hash, Hashable},
    operator::OperatorId,
    serialization::encoding::{base64_encoded, hex_encoded},
};

pub mod request;
pub mod status;

pub type ProofId = Blake3Hash;

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Proof {
    #[serde(with = "base64_encoded")]
    pub proof: Vec<u8>,
    #[serde(with = "hex_encoded")]
    pub prover: OperatorId,
}

impl Debug for Proof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let short_proof = self.proof.split_at(10.min(self.proof.len())).0;
        f.write_fmt(format_args!(
            "Proof {{ prover: {}, proof: {}... }}",
            &self.prover.0.encode_hex_with_prefix(),
            short_proof.encode_hex_with_prefix(),
        ))
    }
}

impl Display for Proof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

impl Proof {
    pub fn new(proof: Vec<u8>, prover: OperatorId) -> Self {
        Self { proof, prover }
    }
}

impl Hashable for Proof {
    fn collect(&self) -> Cow<[u8]> {
        [self.proof.as_slice(), self.prover.0.as_ref()]
            .concat()
            .into()
    }
}

#[cfg(test)]
mod tests {
    use ethers::types::H256;

    use super::Proof;
    use crate::{hash::keccak256::Keccak256Hash, operator::OperatorId};

    #[test]
    fn test_serialization() {
        let prs = vec![Proof {
            proof: vec![1, 2, 3, 4, 5, 6, 7, 8],
            prover: OperatorId::from(Keccak256Hash(H256::from_slice(
                vec![
                    0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1,
                    2, 3, 4, 5, 6, 7,
                ]
                .as_slice(),
            ))),
        }];

        let proof_str = serde_json::to_string_pretty(&prs).unwrap();
        println!("{}", proof_str);

        let proof_json: Vec<Proof> = serde_json::from_str(&proof_str).unwrap();
        assert_eq!(prs, proof_json);
        println!("{:?}", proof_json);

        let bincode_ser = bincode::serialize(&proof_json).unwrap();
        let bincode_deser = bincode::deserialize::<Vec<Proof>>(&bincode_ser).unwrap();

        assert_eq!(bincode_deser, proof_json)
    }
}
