#[cfg(feature = "send_proof_requests")]
use std::time::Duration;

use clap::{Parser, Subcommand};
use ethers::{prelude::U256, types::Address};
use fermah_common::{
    crypto::keystore::KeystoreConfig,
    proof::request::ProofRequest,
    types::network::Connection,
};
use fermah_config::{
    keystore::command::KeyCommands,
    profile::{
        command::{MergableArgs, ProfileCommands},
        key::ProfileKey,
    },
};
use serde::{Deserialize, Serialize};
use url::Url;

/// Arguments for proof request configuration
#[derive(Serialize, Deserialize, Parser, Debug)]
pub struct ProofRequestProfileArgs {}

impl MergableArgs for ProofRequestProfileArgs {
    type Error = ();
    type MergeType = ProofRequest;

    async fn merge(&self, other: Self::MergeType) -> Result<Self::MergeType, Self::Error> {
        Ok(other)
    }
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Proof requests configuration
    Proof {
        #[command(subcommand)]
        profiles: ProfileCommands<ProofRequestProfileArgs>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ImageCommands {
    /// Serve images from local directory
    Serve {
        /// File directory, defaults to ~/.fermah/images
        #[arg(long)]
        dir: Option<String>,

        /// Port to serve image on
        #[arg(long, default_value = "3000")]
        port: u16,
    },
    /// Download image from remote URL and set it to a proof request
    Download {
        /// Docker image name
        #[arg(long)]
        image_name: String,

        /// Docker image version
        #[arg(long, default_value = "latest")]
        version: String,

        /// URL to download image from
        #[arg(long)]
        from: String,

        /// URL to set image fetch to, optional, defaults to "from" url
        #[arg(long)]
        url: Option<String>,

        /// Set this image as prover
        #[arg(long, default_value_t = true)]
        prover: bool,

        /// Set this image as verifier
        #[arg(long, default_value_t = true)]
        verifier: bool,

        #[command(flatten)]
        proof_request_profile: ProfileKey,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProofCommands {
    /// Send Proof Request
    #[command(alias = "send")]
    SendProofRequest {
        #[command(flatten)]
        profile_key: ProfileKey,
        /// Matchmaker RPC connection
        #[arg(long, value_parser = Connection::try_from_str)]
        rpc: Option<Connection>,
        #[command(flatten)]
        key: KeystoreConfig,
    },
    #[cfg(feature = "send_proof_requests")]
    /// Send One Proof Request every N seconds
    SendProofRequests {
        #[command(flatten)]
        profile_key: ProfileKey,
        /// Matchmaker RPC connection
        #[arg(long, value_parser = Connection::try_from_str)]
        rpc: Option<Connection>,
        #[command(flatten)]
        key: KeystoreConfig,
        /// Initial nonce value
        #[arg(long)]
        nonce: Option<u64>,
        /// Pause duration between two proof requests (humantime format)
        #[arg(long, value_parser = humantime::parse_duration, default_value = "30s")]
        pause: Duration,
    },
    /// Check submitted Proof Request's status
    #[command(alias = "check")]
    CheckProofRequest {
        #[command(flatten)]
        profile_key: ProfileKey,
        /// Matchmaker RPC connection
        #[arg(long, value_parser = Connection::try_from_str)]
        rpc: Option<Connection>,
        #[command(flatten)]
        key: KeystoreConfig,
        /// Proof request ID
        #[arg(long)]
        id: String,
        /// Output directory
        #[arg(long)]
        out_dir: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ClientCommands {
    /// Setup CLI configuration
    #[command(alias = "cfg")]
    Config {
        #[command(subcommand)]
        configs: ConfigCommands,
    },
    /// Crypto keypair management
    Key {
        #[command(subcommand)]
        keys: KeyCommands,
    },
    #[command(alias = "img")]
    Image {
        #[command(subcommand)]
        images: ImageCommands,
    },
    Proof {
        #[command(subcommand)]
        proofs: ProofCommands,
    },
    /// Deposit into the AVS vault
    Deposit {
        /// Matchmaker RPC connection
        #[arg(long, value_parser = Connection::try_from_str)]
        rpc: Option<Connection>,
        /// Chain RPC connection
        #[arg(long, default_value = "http://127.0.0.1:8545")]
        chain_rpc: Url,
        #[command(flatten)]
        key: KeystoreConfig,
        #[cfg(feature = "mint_vault_token")]
        /// Minter keystore
        #[arg(long)]
        minter_key: String,
        #[command(flatten)]
        avs_profile: ProfileKey,
        /// Amount to mint
        #[arg(long, value_parser = U256::from_dec_str)]
        amount: U256,
        /// With approval
        #[arg(long)]
        with_approval: bool,
        /// Recipient address in Vault contract. If not provided - use sender address
        #[arg(short = 'a', long)]
        address: Option<Address>,
    },
    /// Update AVS vault with client's balance
    #[command(alias = "update")]
    UpdateBalance {
        #[command(flatten)]
        profile_key: ProfileKey,
        /// Matchmaker RPC connection
        #[arg(long, value_parser = Connection::try_from_str)]
        rpc: Option<Connection>,
        #[command(flatten)]
        key: KeystoreConfig,
    },
    /// Return unspent funds
    #[command(alias = "return")]
    ReturnUnspent {
        #[command(flatten)]
        profile_key: ProfileKey,
        /// Matchmaker RPC connection
        #[arg(long, value_parser = Connection::try_from_str)]
        rpc: Option<Connection>,
        #[command(flatten)]
        key: KeystoreConfig,
    },
}
