use std::{io, io::Read, path::Path};

use clap::Subcommand;
use const_hex::{traits::FromHex, ToHexExt};
use fermah_common::{
    cli::{
        self,
        prompts::print_var,
        spinner::{Spinner, SpinnerTemplate},
    },
    crypto::{
        cipher::{aes128ctr::Aes128CtrCipher, plain::PlainCipher, Cipher},
        kdf::scrypt::ScryptKdf,
        keystore::{KeystoreCipher, KeystoreFile, KEYS_DIR},
        signer::{bls::BlsSigner, ecdsa::EcdsaSigner, Signer, SignerType},
    },
    fs::{self, ensure_dir, json::Json},
};
use rand_core::OsRng;
use termion::color;
use tracing::info;
use uuid::Uuid;

use crate::keystore::{error::Error, KeyArgs, PasswordArgs};

#[derive(Subcommand, Debug)]
pub enum KeyCommands {
    /// Import a private key
    Import {
        #[command(flatten)]
        key: KeyArgs,
        /// A name for the key, will be used as its ID
        #[arg(long)]
        name: String,
    },
    /// Generate a key pair
    Gen {
        #[command(flatten)]
        pw: PasswordArgs,
        #[arg(long)]
        key_type: SignerType,
        /// A name for the key, will be used as its ID
        #[arg(long)]
        name: String,
    },
}

impl KeyCommands {
    pub async fn run(&self) -> Result<(), Error> {
        let home = fs::app_home_dir().await?;
        let keys_dir = home.join(KEYS_DIR);
        ensure_dir(&keys_dir, Some(0o700)).await?;

        match self {
            KeyCommands::Import { key: args, name } => {
                info!(?args.key_type, "importing");

                let key_data = tokio::fs::read_to_string(&args.private_key)
                    .await
                    .unwrap_or_else(|_| args.private_key.clone());

                match args.key_type {
                    SignerType::Ecdsa => {
                        let (address, private_key) =
                            Self::get_keypair::<EcdsaSigner>(Vec::from_hex(key_data.trim())?)?;
                        Self::save_keys(&keys_dir, private_key, address, name.as_str(), &args.pw)
                            .await?;
                    }
                    SignerType::BLS => {
                        let (address, private_key) =
                            Self::get_keypair::<BlsSigner>(Vec::from_hex(key_data.trim())?)?;
                        Self::save_keys(&keys_dir, private_key, address, name.as_str(), &args.pw)
                            .await?;
                    }
                }

                Ok(())
            }
            KeyCommands::Gen { pw, key_type, name } => {
                info!(?key_type, "generating");

                match key_type {
                    SignerType::Ecdsa => {
                        let (address, private_key) = Self::get_random_keypair::<EcdsaSigner>()?;
                        Self::save_keys(&keys_dir, private_key, address, name.as_str(), pw).await?;
                    }
                    SignerType::BLS => {
                        let (address, private_key) = Self::get_random_keypair::<BlsSigner>()?;
                        Self::save_keys(&keys_dir, private_key, address, name.as_str(), pw).await?;
                    }
                }

                Ok(())
            }
        }
    }

    fn get_keypair<S: Signer>(private_key: Vec<u8>) -> Result<(Vec<u8>, Vec<u8>), Error>
    where
        Error: From<<S as Signer>::SignerError>,
    {
        let signer = S::from_bytes(private_key.as_slice())?;
        Ok((signer.public_address(), private_key))
    }

    fn get_random_keypair<S: Signer>() -> Result<(Vec<u8>, Vec<u8>), Error>
    where
        Error: From<<S as Signer>::SignerError>,
    {
        let (signer, private_key) = S::from_random(&mut OsRng)?;
        Ok((signer.public_address(), private_key))
    }

    async fn save_keys(
        keys_dir: &Path,
        private_key: Vec<u8>,
        address: Vec<u8>,
        name: &str,
        pw_args: &PasswordArgs,
    ) -> Result<(), Error> {
        let uuid = Uuid::new_v4();
        let key_file = keys_dir.join(format!("{}.json", name));

        if key_file.exists() {
            return Err(Error::KeystoreExists(
                key_file.to_string_lossy().to_string(),
            ));
        }

        let password = if pw_args.no_password {
            String::default()
        } else {
            match &pw_args.password_stdin {
                true => {
                    let mut pw = String::new();
                    io::stdin().read_to_string(&mut pw)?;
                    pw.trim().to_string()
                }
                false => Self::prompt_password(pw_args)?,
            }
        };

        match password.is_empty() {
            false => {
                let mut cipher = Aes128CtrCipher::<ScryptKdf>::from_data(private_key);

                let spinner = Spinner::new(1, "🔒 Encrypting", SpinnerTemplate::Default);

                cipher.encrypt(password.as_bytes()).unwrap();

                spinner.finish("Done!", true);

                let keystore = KeystoreCipher::new(cipher, address.clone(), uuid, name.to_string());
                info!(?key_file, "saving encrypted private");

                KeystoreFile::Encrypted(keystore)
                    .to_json_path(&key_file)
                    .await?;

                print_var("file", key_file.display());
                print_var("address", address.encode_hex_with_prefix());
            }
            true => {
                let keystore = KeystoreCipher::new(
                    PlainCipher::new(private_key),
                    address.clone(),
                    uuid,
                    name.to_string(),
                );
                info!(?key_file, "saving plaintext private");

                KeystoreFile::Plain(keystore)
                    .to_json_path(&key_file)
                    .await?;

                print_var("file", key_file.display());
                print_var("address", address.encode_hex_with_prefix());
            }
        }
        Ok(())
    }

    fn prompt_password(pw_args: &PasswordArgs) -> Result<String, Error> {
        let mut password = "p".to_string();
        let mut confirm = "".to_string();

        while password != confirm {
            password = cli::prompts::prompt_for_password()?;
            if password.is_empty() {
                break;
            }

            if pw_args.no_pw_confirm {
                break;
            }

            confirm = cli::prompts::prompt_for_password_confirmation()?;
            if confirm != password {
                println!(
                    "{}Passwords do not match. Please try again.\n{}",
                    color::Fg(color::Yellow),
                    color::Fg(color::Reset)
                );
            }
        }
        Ok(password)
    }
}
