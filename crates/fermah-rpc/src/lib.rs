use clap::{self, Parser};
use ethers::types::Address;
use fermah_common::{
    crypto::signer::{ecdsa::EcdsaSigner, SignedData},
    hash::blake3::Blake3Hasher,
    proof::{request::ProofRequest, status::ProofStatus},
    serialization::hash::SerializableHash,
    types::network::Connection,
};
use jsonrpsee::{
    core::{RpcResult, Serialize},
    proc_macros::rpc,
};
use serde::Deserialize;

#[cfg(feature = "server")]
pub mod metrics;
#[cfg(feature = "client")]
pub mod rpc_client;
#[cfg(feature = "server")]
pub mod rpc_server;
pub mod upstream;

#[derive(Serialize, Deserialize, Parser, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct RpcConfig {
    /// Connection settings for RPC
    #[arg(long, value_parser = Connection::try_from_str, default_value = "127.0.0.1:8080")]
    pub connection: Connection,
}

#[rpc(server, client)]
pub(crate) trait RpcApi {
    #[method(name = "submitProofRequest")]
    async fn submit_proof_request(
        &self,
        proof_request: SignedData<ProofRequest, EcdsaSigner>,
    ) -> RpcResult<()>;

    #[method(name = "checkRequestStatus")]
    async fn check_request_status(
        &self,
        request_status: SignedData<SerializableHash<Blake3Hasher>, EcdsaSigner>,
    ) -> RpcResult<ProofStatus>;

    #[method(name = "updateBalance")]
    async fn update_balance(&self, someone: SignedData<Address, EcdsaSigner>) -> RpcResult<()>;

    #[method(name = "updateRegisteredTillBlock")]
    async fn update_registered_till_block(
        &self,
        someone: SignedData<Address, EcdsaSigner>,
    ) -> RpcResult<()>;

    #[method(name = "returnUnspent")]
    async fn return_unspent(&self, someone: SignedData<Address, EcdsaSigner>) -> RpcResult<()>;

    // Withdraw to operator
    #[method(name = "withdraw")]
    async fn withdraw(&self, someone: SignedData<Address, EcdsaSigner>) -> RpcResult<()>;

    // Health endpoint
    #[method(name = "health")]
    async fn health(&self) -> RpcResult<String>;

    // Nodes Health endpoint
    #[method(name = "nodes")]
    async fn nodes(&self) -> RpcResult<usize>;
}
