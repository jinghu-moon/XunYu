use argh::FromArgs;

/// FileVault v13 foundation commands.
#[derive(FromArgs)]
#[argh(subcommand, name = "vault")]
pub struct VaultCmd {
    #[argh(subcommand)]
    pub cmd: VaultSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
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
#[derive(FromArgs)]
#[argh(subcommand, name = "enc")]
pub struct VaultEncCmd {
    /// source plaintext path
    #[argh(positional)]
    pub input: String,

    /// output ciphertext path (default: <input>.fv)
    #[argh(option, short = 'o')]
    pub output: Option<String>,

    /// password slot value
    #[argh(option)]
    pub password: Option<String>,

    /// keyfile slot path
    #[argh(option)]
    pub keyfile: Option<String>,

    /// import an existing recovery key as a slot
    #[argh(option)]
    pub recovery_key: Option<String>,

    /// generate a new recovery key and write it to the given file
    #[argh(option)]
    pub emit_recovery_key: Option<String>,

    /// add a same-profile Windows DPAPI slot
    #[argh(switch)]
    pub dpapi: bool,

    /// payload algorithm: aes256-gcm|xchacha20-poly1305
    #[argh(option, default = "String::from(\"aes256-gcm\")")]
    pub algo: String,

    /// password/keyfile KDF: argon2id|pbkdf2-sha256
    #[argh(option, default = "String::from(\"argon2id\")")]
    pub kdf: String,

    /// chunk size in bytes
    #[argh(option, default = "262144")]
    pub chunk_size: u32,

    /// print machine-readable json
    #[argh(switch)]
    pub json: bool,
}

/// Decrypt a FileVault v13 ciphertext.
#[derive(FromArgs)]
#[argh(subcommand, name = "dec")]
pub struct VaultDecCmd {
    /// source ciphertext path
    #[argh(positional)]
    pub input: String,

    /// output plaintext path
    #[argh(option, short = 'o')]
    pub output: Option<String>,

    /// unlock with password
    #[argh(option)]
    pub password: Option<String>,

    /// unlock with keyfile path
    #[argh(option)]
    pub keyfile: Option<String>,

    /// unlock with recovery key text
    #[argh(option)]
    pub recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[argh(switch)]
    pub dpapi: bool,

    /// print machine-readable json
    #[argh(switch)]
    pub json: bool,
}

/// Inspect FileVault v13 structure and slot metadata.
#[derive(FromArgs)]
#[argh(subcommand, name = "inspect")]
pub struct VaultInspectCmd {
    /// ciphertext path
    #[argh(positional)]
    pub path: String,

    /// print json instead of human-readable text
    #[argh(switch)]
    pub json: bool,
}

/// Verify FileVault v13 integrity without exporting plaintext.
#[derive(FromArgs)]
#[argh(subcommand, name = "verify")]
pub struct VaultVerifyCmd {
    /// ciphertext path
    #[argh(positional)]
    pub path: String,

    /// unlock with password for authenticated verification
    #[argh(option)]
    pub password: Option<String>,

    /// unlock with keyfile path for authenticated verification
    #[argh(option)]
    pub keyfile: Option<String>,

    /// unlock with recovery key text for authenticated verification
    #[argh(option)]
    pub recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI for authenticated verification
    #[argh(switch)]
    pub dpapi: bool,

    /// print json instead of human-readable text
    #[argh(switch)]
    pub json: bool,
}

/// Resume an interrupted encryption task from its journal.
#[derive(FromArgs)]
#[argh(subcommand, name = "resume")]
pub struct VaultResumeCmd {
    /// intended final ciphertext path
    #[argh(positional)]
    pub path: String,

    /// unlock with password
    #[argh(option)]
    pub password: Option<String>,

    /// unlock with keyfile path
    #[argh(option)]
    pub keyfile: Option<String>,

    /// unlock with recovery key text
    #[argh(option)]
    pub recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[argh(switch)]
    pub dpapi: bool,

    /// print machine-readable json
    #[argh(switch)]
    pub json: bool,
}

/// Remove FileVault temporary artifacts.
#[derive(FromArgs)]
#[argh(subcommand, name = "cleanup")]
pub struct VaultCleanupCmd {
    /// intended final ciphertext path
    #[argh(positional)]
    pub path: String,

    /// print machine-readable json
    #[argh(switch)]
    pub json: bool,
}

/// Replace wrapped slots without re-encrypting payload.
#[derive(FromArgs)]
#[argh(subcommand, name = "rewrap")]
pub struct VaultRewrapCmd {
    /// ciphertext path
    #[argh(positional)]
    pub path: String,

    /// unlock with current password
    #[argh(option)]
    pub unlock_password: Option<String>,

    /// unlock with current keyfile path
    #[argh(option)]
    pub unlock_keyfile: Option<String>,

    /// unlock with current recovery key text
    #[argh(option)]
    pub unlock_recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[argh(switch)]
    pub unlock_dpapi: bool,

    /// add or replace a password slot
    #[argh(option)]
    pub add_password: Option<String>,

    /// add or replace a keyfile slot
    #[argh(option)]
    pub add_keyfile: Option<String>,

    /// add or replace a recovery-key slot with an existing recovery key
    #[argh(option)]
    pub add_recovery_key: Option<String>,

    /// generate and add a new recovery-key slot, then write it to this file
    #[argh(option)]
    pub emit_recovery_key: Option<String>,

    /// add or replace a DPAPI slot
    #[argh(switch)]
    pub add_dpapi: bool,

    /// remove slots by kind: password|keyfile|recovery-key|dpapi
    #[argh(option)]
    pub remove_slot: Vec<String>,

    /// password/keyfile KDF: argon2id|pbkdf2-sha256
    #[argh(option, default = "String::from(\"argon2id\")")]
    pub kdf: String,

    /// print machine-readable json
    #[argh(switch)]
    pub json: bool,
}

/// Rebuild a recovery-key slot from another legal unlock path.
#[derive(FromArgs)]
#[argh(subcommand, name = "recover-key")]
pub struct VaultRecoverKeyCmd {
    /// ciphertext path
    #[argh(positional)]
    pub path: String,

    /// unlock with current password
    #[argh(option)]
    pub unlock_password: Option<String>,

    /// unlock with current keyfile path
    #[argh(option)]
    pub unlock_keyfile: Option<String>,

    /// unlock with current recovery key text
    #[argh(option)]
    pub unlock_recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[argh(switch)]
    pub unlock_dpapi: bool,

    /// output file to receive the regenerated recovery key
    #[argh(option)]
    pub output: String,

    /// print machine-readable json
    #[argh(switch)]
    pub json: bool,
}
