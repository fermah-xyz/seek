use std::{
    net::SocketAddr,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use ethers::types::Address;
use fermah_common::{
    crypto::signer::{ecdsa::EcdsaSigner, SignedData},
    hash::blake3::Blake3Hasher,
    proof::{request::ProofRequest, status::ProofStatus},
    serialization::hash::SerializableHash,
};
#[cfg(feature = "db")]
use fermah_database::Database;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    server::{Server, ServerHandle},
    types::{ErrorCode, ErrorObject},
};
use tokio::sync::{mpsc::Sender, Mutex};
use tracing::{debug, error, info};

use crate::{metrics::Metrics, upstream::UpstreamEvent, RpcApiServer, RpcConfig};

#[derive(Debug)]
struct CachedValue<T> {
    value: Option<T>,
    last_updated: Instant,
}

#[derive(Debug, Clone)]
pub struct RpcServer {
    config: RpcConfig,
    pub proof_request_tx: Option<Sender<UpstreamEvent>>,
    #[cfg(feature = "db")]
    db: Database,
    nodes: Arc<Mutex<CachedValue<usize>>>,
}

impl RpcServer {
    /// Create a RPC server from config.
    pub fn new(config: RpcConfig, #[cfg(feature = "db")] db: Database) -> Self {
        Self {
            config,
            proof_request_tx: None,
            #[cfg(feature = "db")]
            db,
            nodes: Arc::new(Mutex::new(CachedValue {
                value: None,
                last_updated: Instant::now() - Duration::from_secs(61),
            })),
        }
    }

    pub async fn spawn_and_run(
        &mut self,
        proof_request_tx: Sender<UpstreamEvent>,
    ) -> Result<ServerHandle> {
        let addr: SocketAddr = self.config.connection.into();

        let server = Server::builder()
            .set_tcp_no_delay(true)
            .build(&addr)
            .await
            .context("failed to start rpc server")?;

        info!("Starting RPC server on {}", addr);

        self.proof_request_tx = Some(proof_request_tx);

        let s: RpcServer = self.clone();
        Ok(server.start(s.into_rpc()))
    }
}

static METRICS: LazyLock<Metrics> = LazyLock::new(Metrics::init);

macro_rules! verify_signature {
    ($request:ident) => {
        if let Err(_err) = $request.verify() {
            METRICS.inc_proof_requests($request.public_key, false);
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "invalid payload signature",
                None as Option<&[u8]>,
            ));
        }
        METRICS.inc_proof_requests($request.public_key, true);
    };
}

#[async_trait]
impl RpcApiServer for RpcServer {
    async fn submit_proof_request(
        &self,
        proof_request: SignedData<ProofRequest, EcdsaSigner>,
    ) -> RpcResult<()> {
        let request_id = proof_request.hash;

        debug!(id=?request_id, "submit_proof_request");
        verify_signature!(proof_request);

        if proof_request.payload.requester.unwrap() != proof_request.public_key {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "Requester is not the signer",
                None as Option<&[u8]>,
            ));
        }

        /*
        // TODO: move this check outside this function as it takes time. The request may timeout.
        let Ok(prover_fname) = proof_request
            .payload
            .download_prover(request_id, &self.config.image_directory)
            .await
        else {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "invalid prover image",
                None as Option<&[u8]>,
            ));
        };
        debug!("prover image hashes match");

        // TODO: move this check to the prover.
        let Ok(verifier_fname) = proof_request
            .payload
            .download_verifier(request_id, &self.config.image_directory)
            .await
        else {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "invalid verifier image",
                None as Option<&[u8]>,
            ));
        };
        debug!("verifier image hashes match");

        debug!(?prover_fname, ?verifier_fname, "Images downloaded");
        */

        if let Err(err) = self
            .proof_request_tx
            .as_ref()
            .expect("Started handling before initiation")
            .send(UpstreamEvent::ProofRequest(proof_request))
            .await
        {
            error!(?err, "failed to send proof request to match maker");
            return Err(ErrorObject::owned(
                ErrorCode::ServerIsBusy.code(),
                "can't handle the proof request",
                None as Option<&[u8]>,
            ));
        }
        debug!("Proof request sent over the chanel");
        Ok(())
    }

    async fn check_request_status(
        &self,
        request_status: SignedData<SerializableHash<Blake3Hasher>, EcdsaSigner>,
    ) -> RpcResult<ProofStatus> {
        debug!(
            "check_request_status for request {:?}",
            request_status.payload.0
        );
        verify_signature!(request_status);

        #[cfg(feature = "db")]
        if let Some(pr) = self
            .db
            .get_proof_request(&request_status.payload.0)
            .map_err(|err| {
                error!(?err, id=?request_status.payload.0, "failed to check request status: database internal error");
                ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    "database internal error",
                    None as Option<&[u8]>,
                )
            })?
        {
            info!(id=?request_status.payload.0, status=?pr.status, "check_request_status");
            return Ok(pr.status);
        }
        #[cfg(not(feature = "db"))]
        panic!("To make this handle work, you need to turn on 'db' feature");

        return Err(ErrorObject::owned(
            ErrorCode::InvalidParams.code(),
            "unknown proof request",
            None as Option<&[u8]>,
        ));
    }

    async fn update_balance(&self, someone: SignedData<Address, EcdsaSigner>) -> RpcResult<()> {
        debug!(addr=?someone, "update_balance request");
        verify_signature!(someone);

        if someone.payload != someone.public_key {
            return Err(ErrorObject::owned(
                ErrorCode::ServerIsBusy.code(),
                "For now only the signer can should send this request",
                None as Option<&[u8]>,
            ));
        }

        if let Err(err) = self
            .proof_request_tx
            .as_ref()
            .expect("Started handling before initiation")
            .send(UpstreamEvent::UpdateBalance(someone.payload))
            .await
        {
            error!(?err, "failed to send update_balance request to match maker");
            return Err(ErrorObject::owned(
                ErrorCode::ServerIsBusy.code(),
                "can't handle the update_balance request",
                None as Option<&[u8]>,
            ));
        }

        Ok(())
    }

    async fn update_registered_till_block(
        &self,
        someone: SignedData<Address, EcdsaSigner>,
    ) -> RpcResult<()> {
        debug!(addr=?someone, "update_registered_till_block request");
        verify_signature!(someone);
        if someone.payload != someone.public_key {
            return Err(ErrorObject::owned(
                ErrorCode::ServerIsBusy.code(),
                "For now only the signer can send this request",
                None as Option<&[u8]>,
            ));
        }

        if let Err(err) = self
            .proof_request_tx
            .as_ref()
            .expect("Started handling before initiation")
            .send(UpstreamEvent::UpdateRegisteredTillBlock(someone.payload))
            .await
        {
            error!(
                ?err,
                "failed to send update_registered_till_block request to match maker"
            );
            return Err(ErrorObject::owned(
                ErrorCode::ServerIsBusy.code(),
                "can't handle the update_registered_till_block request",
                None as Option<&[u8]>,
            ));
        }

        Ok(())
    }

    async fn return_unspent(&self, someone: SignedData<Address, EcdsaSigner>) -> RpcResult<()> {
        debug!(addr=?someone, "return_unspent request");
        verify_signature!(someone);
        if someone.payload != someone.public_key {
            return Err(ErrorObject::owned(
                ErrorCode::ServerIsBusy.code(),
                "For now only the signer can should send this request",
                None as Option<&[u8]>,
            ));
        }

        if let Err(err) = self
            .proof_request_tx
            .as_ref()
            .expect("Started handling before initiation")
            .send(UpstreamEvent::ReturnUnspent(someone.payload))
            .await
        {
            error!(?err, "failed to send return_unspent request to match maker");
            return Err(ErrorObject::owned(
                ErrorCode::ServerIsBusy.code(),
                "can't handle the return_unspent request",
                None as Option<&[u8]>,
            ));
        }

        Ok(())
    }

    async fn withdraw(&self, someone: SignedData<Address, EcdsaSigner>) -> RpcResult<()> {
        debug!(addr=?someone, "withdraw request");
        verify_signature!(someone);
        if someone.payload != someone.public_key {
            return Err(ErrorObject::owned(
                ErrorCode::ServerIsBusy.code(),
                "For now only the signer can should send this request",
                None as Option<&[u8]>,
            ));
        }

        if let Err(err) = self
            .proof_request_tx
            .as_ref()
            .expect("Started handling before initiation")
            .send(UpstreamEvent::Withdraw(someone.payload))
            .await
        {
            error!(?err, "failed to send withdraw request to match maker");
            return Err(ErrorObject::owned(
                ErrorCode::ServerIsBusy.code(),
                "can't handle the withdraw request",
                None as Option<&[u8]>,
            ));
        }

        Ok(())
    }

    /// Example POST request:
    /// {
    ///   "method": "health",
    ///   "params": [],
    ///   "id": 1,
    ///   "jsonrpc": "2.0"
    /// }
    async fn health(&self) -> RpcResult<String> {
        Ok("ok".to_string())
    }

    /// Example POST request:
    /// {
    ///   "method": "nodes",
    ///   "params": [],
    ///   "id": 1,
    ///   "jsonrpc": "2.0"
    /// }
    async fn nodes(&self) -> RpcResult<usize> {
        let mut nodes = self.nodes.lock().await;

        if nodes.last_updated.elapsed() < Duration::from_secs(60) {
            if let Some(cached_ops) = nodes.value {
                return Ok(cached_ops);
            }
        }

        // If cache is outdated or empty, fetch new data
        let ops = self.db.available_operators().unwrap_or_default();
        let ops_len = ops.len();

        // Update cache
        nodes.value = Some(ops_len);
        nodes.last_updated = Instant::now();

        Ok(ops_len)
    }
}
