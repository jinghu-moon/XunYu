use argh::FromArgs;

#[cfg(feature = "crypt")]
/// Encrypt a file using Windows EFS (or other providers).
#[derive(FromArgs)]
#[argh(subcommand, name = "encrypt")]
pub struct EncryptCmd {
    /// target path
    #[argh(positional)]
    pub path: String,

    /// use Windows EFS encryption (Encrypting File System)
    #[argh(switch)]
    pub efs: bool,

    /// public key to encrypt to (age format, can be repeated)
    #[argh(option)]
    pub to: Vec<String>,

    /// encrypt with a passphrase (interactive)
    #[argh(switch)]
    pub passphrase: bool,

    /// output file path (default: <path>.age if not efs)
    #[argh(option, short = 'o')]
    pub out: Option<String>,
}

#[cfg(feature = "crypt")]
/// Decrypt a file.
#[derive(FromArgs)]
#[argh(subcommand, name = "decrypt")]
pub struct DecryptCmd {
    /// target path
    #[argh(positional)]
    pub path: String,

    /// use Windows EFS decryption
    #[argh(switch)]
    pub efs: bool,

    /// identity file to decrypt with (age format, can be repeated)
    #[argh(option, short = 'i')]
    pub identity: Vec<String>,

    /// decrypt with a passphrase (interactive)
    #[argh(switch)]
    pub passphrase: bool,

    /// output file path (default: remove .age extension if not efs)
    #[argh(option, short = 'o')]
    pub out: Option<String>,
}
