use clap::Args;

#[cfg(feature = "crypt")]
/// Encrypt a file using Windows EFS (or other providers).
#[derive(Args, Debug, Clone)]
pub struct EncryptCmd {
    /// target path
    pub path: String,

    /// use Windows EFS encryption (Encrypting File System)
    #[arg(long)]
    pub efs: bool,

    /// public key to encrypt to (age format, can be repeated)
    #[arg(long)]
    pub to: Vec<String>,

    /// encrypt with a passphrase (interactive)
    #[arg(long)]
    pub passphrase: bool,

    /// output file path (default: <path>.age if not efs)
    #[arg(short = 'o', long)]
    pub out: Option<String>,
}

#[cfg(feature = "crypt")]
/// Decrypt a file.
#[derive(Args, Debug, Clone)]
pub struct DecryptCmd {
    /// target path
    pub path: String,

    /// use Windows EFS decryption
    #[arg(long)]
    pub efs: bool,

    /// identity file to decrypt with (age format, can be repeated)
    #[arg(short = 'i', long)]
    pub identity: Vec<String>,

    /// decrypt with a passphrase (interactive)
    #[arg(long)]
    pub passphrase: bool,

    /// output file path (default: remove .age extension if not efs)
    #[arg(short = 'o', long)]
    pub out: Option<String>,
}
