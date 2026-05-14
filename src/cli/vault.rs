use clap::{Args, Parser, Subcommand};

/// FileVault v13 foundation commands.
#[derive(Parser, Debug, Clone)]
pub struct VaultCmd {
    #[command(subcommand)]
    pub cmd: VaultSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum VaultSubCommand {
    Enc(VaultEncCmd),
    Dec(VaultDecCmd),
    Inspect(VaultInspectCmd),
    Verify(VaultVerifyCmd),
    Resume(VaultResumeCmd),
    Cleanup(VaultCleanupCmd),
    Rewrap(VaultRewrapCmd),
    RecoverKey(VaultRecoverKeyCmd),
}

/// Encrypt a file into FileVault v13 format.
#[derive(Args, Debug, Clone)]
pub struct VaultEncCmd {
    /// source plaintext path
    pub input: String,

    /// output ciphertext path (default: <input>.fv)
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// password slot value
    #[arg(long)]
    pub password: Option<String>,

    /// keyfile slot path
    #[arg(long)]
    pub keyfile: Option<String>,

    /// import an existing recovery key as a slot
    #[arg(long)]
    pub recovery_key: Option<String>,

    /// generate a new recovery key and write it to the given file
    #[arg(long)]
    pub emit_recovery_key: Option<String>,

    /// add a same-profile Windows DPAPI slot
    #[arg(long)]
    pub dpapi: bool,

    /// payload algorithm: aes256-gcm|xchacha20-poly1305
    #[arg(long, default_value = "aes256-gcm")]
    pub algo: String,

    /// password/keyfile KDF: argon2id|pbkdf2-sha256
    #[arg(long, default_value = "argon2id")]
    pub kdf: String,

    /// chunk size in bytes
    #[arg(long, default_value_t = 262144)]
    pub chunk_size: u32,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}

/// Decrypt a FileVault v13 ciphertext.
#[derive(Args, Debug, Clone)]
pub struct VaultDecCmd {
    /// source ciphertext path
    pub input: String,

    /// output plaintext path
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// unlock with password
    #[arg(long)]
    pub password: Option<String>,

    /// unlock with keyfile path
    #[arg(long)]
    pub keyfile: Option<String>,

    /// unlock with recovery key text
    #[arg(long)]
    pub recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[arg(long)]
    pub dpapi: bool,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}

/// Inspect FileVault v13 structure and slot metadata.
#[derive(Args, Debug, Clone)]
pub struct VaultInspectCmd {
    /// ciphertext path
    pub path: String,

    /// print json instead of human-readable text
    #[arg(long)]
    pub json: bool,
}

/// Verify FileVault v13 integrity without exporting plaintext.
#[derive(Args, Debug, Clone)]
pub struct VaultVerifyCmd {
    /// ciphertext path
    pub path: String,

    /// unlock with password for authenticated verification
    #[arg(long)]
    pub password: Option<String>,

    /// unlock with keyfile path for authenticated verification
    #[arg(long)]
    pub keyfile: Option<String>,

    /// unlock with recovery key text for authenticated verification
    #[arg(long)]
    pub recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI for authenticated verification
    #[arg(long)]
    pub dpapi: bool,

    /// print json instead of human-readable text
    #[arg(long)]
    pub json: bool,
}

/// Resume an interrupted encryption task from its journal.
#[derive(Args, Debug, Clone)]
pub struct VaultResumeCmd {
    /// intended final ciphertext path
    pub path: String,

    /// unlock with password
    #[arg(long)]
    pub password: Option<String>,

    /// unlock with keyfile path
    #[arg(long)]
    pub keyfile: Option<String>,

    /// unlock with recovery key text
    #[arg(long)]
    pub recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[arg(long)]
    pub dpapi: bool,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}

/// Remove FileVault temporary artifacts.
#[derive(Args, Debug, Clone)]
pub struct VaultCleanupCmd {
    /// intended final ciphertext path
    pub path: String,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}

/// Replace wrapped slots without re-encrypting payload.
#[derive(Args, Debug, Clone)]
pub struct VaultRewrapCmd {
    /// ciphertext path
    pub path: String,

    /// unlock with current password
    #[arg(long)]
    pub unlock_password: Option<String>,

    /// unlock with current keyfile path
    #[arg(long)]
    pub unlock_keyfile: Option<String>,

    /// unlock with current recovery key text
    #[arg(long)]
    pub unlock_recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[arg(long)]
    pub unlock_dpapi: bool,

    /// add or replace a password slot
    #[arg(long)]
    pub add_password: Option<String>,

    /// add or replace a keyfile slot
    #[arg(long)]
    pub add_keyfile: Option<String>,

    /// add or replace a recovery-key slot with an existing recovery key
    #[arg(long)]
    pub add_recovery_key: Option<String>,

    /// generate and add a new recovery-key slot, then write it to this file
    #[arg(long)]
    pub emit_recovery_key: Option<String>,

    /// add or replace a DPAPI slot
    #[arg(long)]
    pub add_dpapi: bool,

    /// remove slots by kind: password|keyfile|recovery-key|dpapi
    #[arg(long)]
    pub remove_slot: Vec<String>,

    /// password/keyfile KDF: argon2id|pbkdf2-sha256
    #[arg(long, default_value = "argon2id")]
    pub kdf: String,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}

/// Rebuild a recovery-key slot from another legal unlock path.
#[derive(Args, Debug, Clone)]
pub struct VaultRecoverKeyCmd {
    /// ciphertext path
    pub path: String,

    /// unlock with current password
    #[arg(long)]
    pub unlock_password: Option<String>,

    /// unlock with current keyfile path
    #[arg(long)]
    pub unlock_keyfile: Option<String>,

    /// unlock with current recovery key text
    #[arg(long)]
    pub unlock_recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[arg(long)]
    pub unlock_dpapi: bool,

    /// output file to receive the regenerated recovery key
    #[arg(long)]
    pub output: String,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}
