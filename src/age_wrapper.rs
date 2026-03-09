use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use age::x25519::Recipient;
use age::{Decryptor, Encryptor, secrecy::SecretString};

use crate::output::ProgressReporter;

#[cfg(target_os = "windows")]
use crate::windows::safety::ensure_safe_target;

pub enum AgeError {
    Io(io::Error),
    Encrypt(age::EncryptError),
    Decrypt(age::DecryptError),
    SafetyRestricted(&'static str),
    Interactive(String),
}

impl std::fmt::Display for AgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgeError::Io(e) => write!(f, "IO Error: {}", e),
            AgeError::Encrypt(e) => write!(f, "Encryption Error: {}", e),
            AgeError::Decrypt(e) => write!(f, "Decryption Error: {}", e),
            AgeError::SafetyRestricted(msg) => write!(f, "Safety Restriction: {}", msg),
            AgeError::Interactive(msg) => write!(f, "Interactive Error: {}", msg),
        }
    }
}

impl From<io::Error> for AgeError {
    fn from(err: io::Error) -> Self {
        AgeError::Io(err)
    }
}
impl From<age::EncryptError> for AgeError {
    fn from(err: age::EncryptError) -> Self {
        AgeError::Encrypt(err)
    }
}
impl From<age::DecryptError> for AgeError {
    fn from(err: age::DecryptError) -> Self {
        AgeError::Decrypt(err)
    }
}

/// Helper function to copy data stream with progress
fn copy_stream_with_progress<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    total_size: u64,
    msg: &str,
) -> io::Result<()> {
    let pr = ProgressReporter::new(total_size, msg);
    let mut buffer = [0u8; 64 * 1024]; // 64KB buffer
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n])?;
        pr.inc(n as u64);
    }
    pr.finish_with_message("Done");
    Ok(())
}

fn enforce_safety(path: &Path) -> Result<(), AgeError> {
    #[cfg(target_os = "windows")]
    {
        if let Err(e) = ensure_safe_target(path) {
            return Err(AgeError::SafetyRestricted(e));
        }
    }
    Ok(())
}

pub fn encrypt_with_passphrase(
    in_path: &Path,
    out_path: &Path,
    pass: SecretString,
) -> Result<(), AgeError> {
    enforce_safety(in_path)?;
    enforce_safety(out_path)?;

    let in_file = File::open(in_path)?;
    let total_size = in_file.metadata()?.len();
    let out_file = File::create(out_path)?;

    let encryptor = Encryptor::with_user_passphrase(pass);
    let mut writer = encryptor.wrap_output(out_file).map_err(AgeError::Io)?;

    copy_stream_with_progress(in_file, &mut writer, total_size, "Encrypting (pass)")?;
    writer.finish().map_err(AgeError::Io)?;

    Ok(())
}

pub fn encrypt_to_recipients(
    in_path: &Path,
    out_path: &Path,
    pubkeys: Vec<String>,
) -> Result<(), AgeError> {
    enforce_safety(in_path)?;
    enforce_safety(out_path)?;

    let mut recipients: Vec<Box<dyn age::Recipient>> = vec![];
    for pk in pubkeys {
        let parsed = pk
            .parse::<Recipient>()
            .map_err(|_| AgeError::Interactive(format!("Invalid public key: {}", pk)))?;
        recipients.push(Box::new(parsed));
    }

    let in_file = File::open(in_path)?;
    let total_size = in_file.metadata()?.len();
    let out_file = File::create(out_path)?;

    let encryptor =
        Encryptor::with_recipients(recipients.iter().map(|r| r.as_ref() as &dyn age::Recipient))
            .map_err(AgeError::Encrypt)?;

    let mut writer = encryptor.wrap_output(out_file).map_err(AgeError::Io)?;

    copy_stream_with_progress(in_file, &mut writer, total_size, "Encrypting (pubkey)")?;
    writer.finish().map_err(AgeError::Io)?;

    Ok(())
}

pub fn decrypt_file(
    in_path: &Path,
    out_path: &Path,
    passphrase: Option<SecretString>,
    identities_paths: Vec<String>,
) -> Result<(), AgeError> {
    enforce_safety(in_path)?;
    enforce_safety(out_path)?;

    let in_file = File::open(in_path)?;
    let total_size = in_file.metadata()?.len(); // Note: Encrypted size > decrypted size, progress is approximate.

    // Read the age header
    let decryptor = Decryptor::new(in_file)?;

    let reader = if decryptor.is_scrypt() {
        if let Some(pass) = passphrase {
            let identity = age::scrypt::Identity::new(pass);
            decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))?
        } else {
            return Err(AgeError::Interactive(
                "File requires a passphrase, but none was provided (-p).".into(),
            ));
        }
    } else {
        if identities_paths.is_empty() {
            return Err(AgeError::Interactive(
                "File is recipient-encrypted, but no identity (-i) provided.".into(),
            ));
        }

        let mut identities: Vec<Box<dyn age::Identity>> = vec![];
        for path_str in identities_paths {
            let ids = age::IdentityFile::from_file(path_str.clone()).map_err(|e| {
                AgeError::Interactive(format!(
                    "Failed to parse identity file '{}': {}",
                    path_str, e
                ))
            })?;
            let identities_list = ids
                .into_identities()
                .map_err(|e| AgeError::Interactive(format!("Invalid identities: {}", e)))?;
            identities.extend(identities_list);
        }

        let identity_refs: Vec<&dyn age::Identity> = identities
            .iter()
            .map(|b| b.as_ref() as &dyn age::Identity)
            .collect();
        decryptor.decrypt(identity_refs.into_iter())?
    };

    let out_file = File::create(out_path)?;
    copy_stream_with_progress(reader, out_file, total_size, "Decrypting")?;

    Ok(())
}
