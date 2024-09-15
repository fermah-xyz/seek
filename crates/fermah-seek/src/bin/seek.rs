use std::{ops::Add, path::Path};

use anyhow::Context;
use clap::Parser;
use const_hex::{traits::FromHex, ToHexExt};
use fermah_avs::contract::Contracts;
use fermah_common::{
    cli,
    cli::{
        prompts::print_var,
        spinner::{Spinner, SpinnerLayer, SpinnerTemplate},
    },
    crypto::{
        keystore::{KeystoreConfig, KeystoreFile},
        signer::ecdsa::EcdsaSigner,
    },
    executable::Image,
    fs::{app_home_dir, ensure_dir, hash::hash_path, json::Json},
    hash::blake3::Blake3Hasher,
    http::{file_download::FileDownload, file_server::FileServer},
    print_info,
    proof::{request::ProofRequest, status::ProofStatus},
    resources::RemoteResource,
    serialization::hash::SerializableHash,
};
use fermah_config::profile::{FromProfile, Profile, ProfileType, CONFIG_DIR};
use fermah_rpc::{rpc_client::RpcClient, RpcConfig};
use fermah_seek::{
    command::{ClientCommands, ConfigCommands, ImageCommands, ProofCommands},
    error::Error,
    IMAGES_DIR,
    PROOFS_DIR,
};
use fermah_telemetry::{stdout::StdoutTelemetry, Telemetry};
use tracing::{error, info};
use url::Url;

/// Proof Requester CLI
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Commands
    #[command(subcommand)]
    pub command: ClientCommands,
}

#[tokio::main]
async fn main() {
    cli::ascii::print_ascii();
    print_info!();

    let _ = run().await.inspect_err(|e| {
        error!("CLI failed: {e}");
    });
}

async fn run() -> Result<(), Error> {
    let t = StdoutTelemetry::default();
    let cli = Cli::parse();

    let config_dir = app_home_dir().await?.join(CONFIG_DIR);

    match cli.command {
        ClientCommands::Config { configs } => {
            match configs {
                ConfigCommands::Proof { profiles } => {
                    profiles.run(ProfileType::Proof, &config_dir).await?;
                }
            }
        }
        ClientCommands::Image { images } => {
            match images {
                ImageCommands::Serve { dir, port } => {
                    t.init();

                    let d = match dir {
                        Some(d) => d,
                        None => {
                            app_home_dir()
                                .await?
                                .join(IMAGES_DIR)
                                .to_string_lossy()
                                .to_string()
                        }
                    };

                    FileServer::new(port)
                        .serve_dir("images".to_string(), d.into())
                        .await;
                }
                ImageCommands::Download {
                    image_name,
                    version,
                    from,
                    url,
                    prover,
                    verifier,
                    proof_request_profile,
                } => {
                    t.init();

                    let from = Url::parse(&from)?;
                    let dir = app_home_dir().await?.join(IMAGES_DIR);
                    ensure_dir(&dir, None).await?;
                    let filepath = dir.join(image_name.as_str());
                    if !filepath.exists() {
                        download_file(&from, &filepath).await?;
                    }

                    let hash = hash_path::<Blake3Hasher>(&filepath).await?;

                    let mut proof_profile = Profile::<ProofRequest>::from_props(
                        &config_dir,
                        ProfileType::Proof,
                        &proof_request_profile,
                    )
                    .await?;

                    let url = match url {
                        Some(u) => Url::parse(&u)?,
                        None => from.clone(),
                    };

                    let v = format!(":{}", version);

                    if prover {
                        proof_profile.config.prover.image = Image::RemoteDocker((
                            RemoteResource {
                                url: url.clone(),
                                hash,
                            },
                            image_name.clone().add(&v),
                        ));
                    }

                    if verifier {
                        proof_profile.config.verifier.image =
                            Image::RemoteDocker((RemoteResource { url, hash }, image_name.add(&v)));
                    }

                    proof_profile.save().await?;

                    print_var("image", filepath.display());
                    print_var("hash", hash);
                }
            }
        }
        ClientCommands::Key { keys } => {
            t.with_filter("warn".into()).init();

            keys.run().await?;
        }
        ClientCommands::Proof { proofs } => {
            let spinner = Spinner::new(1, "Sending proof request", SpinnerTemplate::Default);

            t.with_spinner_layer(SpinnerLayer::new(
                StdoutTelemetry::default_fmt_layer(),
                spinner.clone(),
            ))
            .init();

            match proofs {
                ProofCommands::SendProofRequest {
                    profile_key,
                    rpc,
                    key,
                } => {
                    let ecdsa_signer = KeystoreFile::from_config(&key)
                        .await?
                        .to_signer::<EcdsaSigner>()
                        .await?;

                    let conn = rpc.unwrap_or_else(|| profile_key.network.to_mm_rpc());

                    let rpc = RpcClient::from_config(RpcConfig { connection: conn }, ecdsa_signer)
                        .await?;

                    let proof_request =
                        ProofRequest::from_profile(&config_dir, ProfileType::Proof, &profile_key)
                            .await?;

                    let proof_request_id = rpc
                        .submit_proof_request(proof_request.clone())
                        .await
                        .inspect_err(|_| {
                            spinner.finish("Failed!", false);
                        })?;

                    spinner.finish("Done!", true);

                    print_var("proof_id", proof_request_id.encode_hex_with_prefix());
                }
                ProofCommands::CheckProofRequest {
                    profile_key,
                    rpc,
                    key,
                    id,
                    out_dir,
                } => {
                    let ecdsa_signer = KeystoreFile::from_config(&key)
                        .await?
                        .to_signer::<EcdsaSigner>()
                        .await?;

                    let conn = rpc.unwrap_or_else(|| profile_key.network.to_mm_rpc());
                    let rpc = RpcClient::from_config(RpcConfig { connection: conn }, ecdsa_signer)
                        .await?;

                    match SerializableHash::from_hex(id.clone()) {
                        Ok(status_request) => {
                            let status = rpc.check_request_status(status_request).await?;
                            if status.is_final() {
                                info!("Proof request is final");
                            }

                            print_var("status", status.to_string());

                            match status {
                                ProofStatus::Proven(proof) => {
                                    let dir = out_dir
                                        .map_or(app_home_dir().await?.join(PROOFS_DIR), |d| {
                                            d.into()
                                        });
                                    ensure_dir(&dir, None).await?;

                                    let filepath = dir.join(format!("{}.json", id));
                                    proof.to_json_path(&filepath).await?;

                                    print_var("proof", filepath.display());
                                }
                                ProofStatus::Rejected(reason) => {
                                    print_var("reason", reason);
                                }
                                ProofStatus::AcknowledgedAssignment(op_id)
                                | ProofStatus::Assigned(op_id) => {
                                    print_var("op_id", op_id.encode_hex_with_prefix());
                                }
                                _ => {}
                            }
                        }
                        Err(err) => {
                            error!(?err, ?id, "Failed to parse proof_request_id")
                        }
                    }
                }
            }
        }

        ClientCommands::Deposit {
            chain_rpc,
            rpc,
            key,
            #[cfg(feature = "mint_vault_token")]
            minter_key,
            avs_profile,
            amount,
            with_approval,
            address,
        } => {
            StdoutTelemetry::default().init();
            let avs = fermah_avs::config::Config::from_profile(
                &config_dir,
                ProfileType::Avs,
                &avs_profile,
            )
            .await?;

            let ecdsa_signer = KeystoreFile::from_config(&key)
                .await?
                .to_signer::<EcdsaSigner>()
                .await?;

            let client_contracts =
                Contracts::from_config(&avs, &chain_rpc, ecdsa_signer.clone()).await?;

            #[cfg(feature = "mint_vault_token")]
            {
                let ecdsa_signer_minter =
                    KeystoreFile::from_config(&KeystoreConfig { key: minter_key })
                        .await?
                        .to_signer::<EcdsaSigner>()
                        .await?;

                let minter_contracts =
                    Contracts::from_config(&avs, &chain_rpc, ecdsa_signer_minter).await?;
                minter_contracts
                    .fermah_contracts
                    .vault_token
                    .mint(client_contracts.provider.address(), amount)
                    .send()
                    .await
                    .inspect_err(|_| tracing::warn!(vault_token=?minter_contracts.fermah_contracts.vault_token.address(), "failed to mint"))?;
            }

            if with_approval {
                client_contracts
                    .fermah_contracts
                    .vault_token
                    .approve(avs.fermah_contract.vault, amount)
                    .send()
                    .await
                    .inspect_err(|_| tracing::warn!(vault_token=?client_contracts.fermah_contracts.vault_token.address() ,"failed to approve"))?
                    .await
                    .context("failed wait for approve")?;
            }
            // If address is not stated in the argument, we fallback to the sender's address
            let address = address.unwrap_or(client_contracts.provider.address());
            let tx = client_contracts
                .fermah_contracts
                .vault
                .deposit(amount, address);
            match tx.send().await {
                Ok(result) => {
                    result.confirmations(1).await.context("failed to confirm")?;
                }
                Err(err) => {
                    error!("failed to wait for confirmation: {err:?}")
                }
            };

            let conn = rpc.unwrap_or_else(|| avs_profile.network.to_mm_rpc());
            RpcClient::from_config(RpcConfig { connection: conn }, ecdsa_signer)
                .await?
                .update_balance()
                .await?;

            info!("Sucessfully deposited {amount} into vault")
        }
        ClientCommands::UpdateBalance {
            profile_key,
            rpc,
            key,
        } => {
            let conn = rpc.unwrap_or_else(|| profile_key.network.to_mm_rpc());

            RpcClient::from_config(
                RpcConfig { connection: conn },
                KeystoreFile::from_config(&key)
                    .await?
                    .to_signer::<EcdsaSigner>()
                    .await?,
            )
            .await?
            .update_balance()
            .await?;
        }
        ClientCommands::ReturnUnspent {
            profile_key,
            rpc,
            key,
        } => {
            let conn = rpc.unwrap_or_else(|| profile_key.network.to_mm_rpc());

            RpcClient::from_config(
                RpcConfig { connection: conn },
                KeystoreFile::from_config(&key)
                    .await?
                    .to_signer::<EcdsaSigner>()
                    .await?,
            )
            .await?
            .return_unspent()
            .await?;
        }
    }

    Ok(())
}

async fn download_file(url: &Url, filepath: &Path) -> Result<(), Error> {
    let spinner = Spinner::new(1, "Downloading image", SpinnerTemplate::Progress);

    let closure_spinner = spinner.clone();
    let progress_callback = move |downloaded_size, total_size| {
        closure_spinner.inner().set_length(total_size);
        closure_spinner.inner().set_position(downloaded_size);

        if total_size == downloaded_size {
            closure_spinner.finish("Done!", true)
        }
    };

    if let Err(e) = FileDownload::new(url.clone())
        .download_to_file(filepath, progress_callback)
        .await
    {
        spinner.finish("Failed!", false);
        error!("failed when downloading image: {}", e);
    }
    Ok(())
}
