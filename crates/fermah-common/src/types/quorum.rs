use ethers::types::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumNumbers<const S: usize>(Vec<u8>);

impl<const S: usize> QuorumNumbers<S> {
    pub fn new(quorum_numbers: Vec<u8>) -> Self {
        if quorum_numbers.len() > S {
            panic!("Quorum numbers must not exceed {S}")
        }

        Self(quorum_numbers)
    }
}

impl<const S: usize> From<QuorumNumbers<S>> for Bytes {
    fn from(value: QuorumNumbers<S>) -> Self {
        value.0.into()
    }
}

impl<const S: usize> From<&[u8]> for QuorumNumbers<S> {
    fn from(value: &[u8]) -> Self {
        let v = Vec::from(value);
        Self::new(v)
    }
}
