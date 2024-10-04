use clap::Parser;
use fermah_common::crypto::signer::SignerType;

pub mod command;
pub mod error;

#[derive(Parser, Debug)]
pub struct PasswordArgs {
    /// Stdin password to encrypt the private key, if not provided it will be prompted
    #[arg(long)]
    pub password_stdin: bool,
    /// Do not ask for password confirmation
    #[arg(long)]
    pub no_pw_confirm: bool,
    /// Do not set a password
    #[arg(long)]
    pub no_password: bool,
    /// Enable fast cipher mode (!INSECURE!)
    #[arg(long)]
    pub fast: bool,
}

#[derive(Parser, Debug)]
pub struct KeyArgs {
    /// Hex encoded private key, i.e. 0x123..
    /// or a relative path to a file containing the hexed private key
    #[arg(long)]
    pub private_key: String,

    #[arg(long)]
    pub key_type: SignerType,

    #[command(flatten)]
    pub pw: PasswordArgs,
}

#[cfg(test)]
mod tests {
    use const_hex::ToHexExt;
    use fermah_common::crypto::{
        keystore::{KeystoreConfig, KeystoreFile},
        signer::{ecdsa::EcdsaSigner, Signer},
    };

    #[tokio::test]
    async fn test_signer_from_keystore() {
        let key = "test".to_string();

        let mut ksfile = KeystoreFile::from_config(&KeystoreConfig { key })
            .await
            .unwrap();

        let ecdsa = ksfile.to_signer::<EcdsaSigner>().await.unwrap();
        assert_eq!(
            ecdsa.public_address().encode_hex_with_prefix(),
            "0x70997970c51812dc3a010c7d01b50e0d17dc79c8"
        );
    }
}
