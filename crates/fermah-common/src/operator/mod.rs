use std::fmt::{Debug, Display};

use const_hex::FromHexError;
use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::crypto::signer::{ecdsa::EcdsaSigner, Signer};

#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Copy)]
pub struct OperatorId(pub <EcdsaSigner as Signer>::VerifyingKey); // #[serde(with = "hex_encoded")]

impl AsRef<Address> for OperatorId {
    fn as_ref(&self) -> &Address {
        &self.0
    }
}

impl AsRef<[u8]> for OperatorId {
    fn as_ref(&self) -> &[u8] {
        &self.0 .0
    }
}

impl From<&[u8]> for OperatorId {
    fn from(value: &[u8]) -> Self {
        OperatorId(Address::from_slice(value))
    }
}

impl From<[u8; 20]> for OperatorId {
    fn from(value: [u8; 20]) -> Self {
        OperatorId(Address::from_slice(&value))
    }
}

impl From<&[u8; 20]> for OperatorId {
    fn from(value: &[u8; 20]) -> Self {
        OperatorId(Address::from_slice(value))
    }
}

impl From<Address> for OperatorId {
    fn from(value: Address) -> Self {
        OperatorId(value)
    }
}

impl From<&Address> for OperatorId {
    fn from(value: &Address) -> Self {
        OperatorId(*value)
    }
}

impl TryFrom<&str> for OperatorId {
    type Error = FromHexError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let decoded = const_hex::decode(value)?;
        Ok(decoded.as_slice().into())
    }
}

impl TryFrom<String> for OperatorId {
    type Error = FromHexError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

impl Display for OperatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self.0)
    }
}

impl Debug for OperatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_fmt() {
        let operator_id = OperatorId::from(&[1; 20]);

        assert_eq!(
            "0x0101010101010101010101010101010101010101",
            operator_id.to_string()
        )
    }

    #[test]
    pub fn test_ivec() {
        let operator_id = OperatorId::from(&[
            144, 247, 155, 246, 235, 44, 79, 135, 3, 101, 231, 133, 152, 46, 31, 16, 30, 147, 185,
            6,
        ]);
        assert_eq!(
            operator_id,
            OperatorId::try_from("0x90f79bf6eb2c4f870365e785982e1f101e93b906").unwrap()
        );
        let op_ivec: &[u8] = operator_id.as_ref();
        let op_id2: OperatorId = op_ivec.into();

        assert_eq!(operator_id, op_id2);
    }
}
