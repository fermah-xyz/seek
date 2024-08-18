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
