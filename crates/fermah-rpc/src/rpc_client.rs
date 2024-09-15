use std::fmt::Debug;

use fermah_common::{
    crypto::signer::{ecdsa::EcdsaSigner, SignedData, Signer},
    hash::blake3::{Blake3Hash, Blake3Hasher},
    proof,
    proof::request::ProofRequest,
    serialization::hash::SerializableHash,
};
use jsonrpsee::{
    async_client::{Client, ClientBuilder},
    client_transport::ws::WsTransportClientBuilder,
};
use tracing::error;

use crate::{RpcApiClient, RpcConfig};

#[derive(Debug, thiserror::Error)]
pub enum RpcClientError {
    #[error("io error: {0}")]
    Fs(#[from] fermah_common::fs::error::Error),

    #[error("RPC client error: {0}")]
    Rpc(#[from] jsonrpsee::core::ClientError),

    #[error("RPC client handshake error: {0}")]
    RpcHandshake(#[from] jsonrpsee::client_transport::ws::WsHandshakeError),

    #[error("proof requester address does not match private key")]
    InvalidRequesterAddress,

    #[error("ecdsa signer error: {0}")]
    EcdsaSigner(#[from] fermah_common::crypto::signer::ecdsa::EcdsaSignerError),

    #[error("keystore file error: {0}")]
    KeystoreError(#[from] fermah_common::crypto::keystore::KeystoreFileError),
}

pub struct RpcClient {
    /// JSON-RPC HTTP client.
    pub client: Client,

    /// RPC Configuration
    pub config: RpcConfig,

    /// Client's signer
    pub signer: EcdsaSigner,
}

impl RpcClient {
    pub async fn from_config(
        config: RpcConfig,
        signer: EcdsaSigner,
    ) -> Result<Self, RpcClientError> {
        let (tx, rx) = WsTransportClientBuilder::default()
            .build(config.connection.into())
            .await
            .inspect_err(|_| error!("failed to connect to RPC server: {}", config.connection))?;

        Ok(Self {
            client: ClientBuilder::default().build_with_tokio(tx, rx),
            config,
            signer,
        })
    }

    pub async fn submit_proof_request(
        &self,
        mut proof_request: ProofRequest,
    ) -> Result<Blake3Hash, RpcClientError> {
        proof_request.requester = Some(self.signer.verifying_key());

        let signed_request = SignedData::new(proof_request, &self.signer)?;
        signed_request.verify()?;

        let proof_request_id = signed_request.hash;

        RpcApiClient::submit_proof_request(&self.client, signed_request).await?;
        Ok(proof_request_id)
    }

    pub async fn check_request_status(
        &self,
        request_status: SerializableHash<Blake3Hasher>,
    ) -> Result<proof::status::ProofStatus, RpcClientError> {
        let signed_request = SignedData::new(request_status, &self.signer)?;
        Ok(RpcApiClient::check_request_status(&self.client, signed_request).await?)
    }

    pub async fn update_balance(&self) -> Result<(), RpcClientError> {
        let address = self.signer.verifying_key();
        let payload = SignedData::new(address, &self.signer)?;
        Ok(RpcApiClient::update_balance(&self.client, payload).await?)
    }

    pub async fn update_registered_till_block(&self) -> Result<(), RpcClientError> {
        let address = self.signer.verifying_key();
        let payload = SignedData::new(address, &self.signer)?;
        Ok(RpcApiClient::update_registered_till_block(&self.client, payload).await?)
    }

    pub async fn return_unspent(&self) -> Result<(), RpcClientError> {
        let address = self.signer.verifying_key();
        let payload = SignedData::new(address, &self.signer)?;
        Ok(RpcApiClient::return_unspent(&self.client, payload).await?)
    }

    pub async fn withdraw(&self) -> Result<(), RpcClientError> {
        let address = self.signer.verifying_key();
        let payload = SignedData::new(address, &self.signer)?;
        Ok(RpcApiClient::withdraw(&self.client, payload).await?)
    }

    pub async fn health(&self) -> Result<String, RpcClientError> {
        Ok(RpcApiClient::health(&self.client).await?)
    }
}
