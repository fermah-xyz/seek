use ethers::types::Address;
use fermah_common::{
    crypto::signer::{ecdsa::EcdsaSigner, SignedData},
    proof::request::ProofRequest,
};

#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // TODO remove me
pub enum UpstreamEvent {
    ProofRequest(SignedData<ProofRequest, EcdsaSigner>),
    UpdateBalance(Address),
    UpdateRegisteredTillBlock(Address),
    ReturnUnspent(Address),
    Withdraw(Address),
}
