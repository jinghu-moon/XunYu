use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use aes_siv::aead::Payload as SivPayload;
use aes_siv::{Aes128SivAead, Nonce as SivNonce};
use argon2::{Algorithm as Argon2Algorithm, Argon2, Params as Argon2Params, Version};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use crc32fast::Hasher as Crc32Hasher;
use getrandom::fill as fill_random;
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

#[cfg(windows)]
use windows::Win32::Foundation::{HLOCAL, LocalFree};
#[cfg(windows)]
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
#[cfg(windows)]
use windows::core::PCWSTR;

type HmacSha256 = Hmac<Sha256>;

const FIXED_HEADER_LEN: usize = 88;
const HEADER_MAC_LEN: usize = 32;
const FRAME_HEADER_LEN: usize = 24;
const FOOTER_LEN: usize = 72;
const MAGIC: &[u8; 8] = b"FILEVA13";
const COMMIT_MAGIC: &[u8; 8] = b"FVCOMMIT";
const FORMAT_VERSION: u16 = 13;
const FLAG_FILENAME_ENCRYPTED: u16 = 0x0001;
const HEADER_MAC_INFO: &[u8] = b"filevault-v13/header-mac";
const FRAME_NONCE_INFO: &[u8] = b"filevault-v13/frame-nonce";
const FILENAME_KEY_INFO: &[u8] = b"filevault-v13/filename-key";

struct StepTiming {
    enabled: bool,
    op: &'static str,
    start: Instant,
    last: Instant,
    steps: Vec<(&'static str, std::time::Duration)>,
}

impl StepTiming {
    fn new(op: &'static str) -> Self {
        let enabled = std::env::var("XUN_FILEVAULT_TIMING")
            .map(|value| {
                let value = value.trim().to_ascii_lowercase();
                matches!(value.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(false);
        let now = Instant::now();
        Self {
            enabled,
            op,
            start: now,
            last: now,
            steps: Vec::new(),
        }
    }

    fn record(&mut self, step: &'static str) {
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        let elapsed = now - self.last;
        self.last = now;
        self.steps.push((step, elapsed));
    }

    fn finish(&self) {
        if !self.enabled {
            return;
        }
        let total = self.start.elapsed();
        eprintln!(
            "perf: filevault timing op={} total_ms={}",
            self.op,
            total.as_millis()
        );
        for (step, elapsed) in &self.steps {
            eprintln!(
                "perf: filevault timing op={} step={} ms={}",
                self.op,
                step,
                elapsed.as_millis()
            );
        }
    }
}

#[derive(Debug)]
struct EncJob {
    sequence: u32,
    plaintext: Vec<u8>,
}

#[derive(Debug)]
struct EncFrame {
    sequence: u32,
    header_bytes: [u8; FRAME_HEADER_LEN],
    ciphertext: Vec<u8>,
}

#[derive(Debug)]
struct DecJob {
    sequence: u32,
    plaintext_len: u32,
    ciphertext: Vec<u8>,
}

#[derive(Debug)]
struct DecFrame {
    sequence: u32,
    plaintext: Vec<u8>,
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn filevault_workers() -> usize {
    let default = thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    env_usize("XUN_FILEVAULT_WORKERS", default).clamp(1, 16)
}

fn filevault_inflight(workers: usize) -> usize {
    let default = workers.saturating_mul(2).max(2);
    env_usize("XUN_FILEVAULT_INFLIGHT", default).max(1)
}

#[derive(Debug)]
pub enum FileVaultError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Crypto(&'static str),
    InvalidFormat(String),
    InvalidArgument(String),
    Verify(String),
    Slot(String),
    Resume(String),
    #[cfg_attr(windows, allow(dead_code))]
    Unsupported(String),
}

impl Display for FileVaultError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Json(err) => write!(f, "json error: {err}"),
            Self::Crypto(msg) => write!(f, "crypto error: {msg}"),
            Self::InvalidFormat(msg) => write!(f, "invalid format: {msg}"),
            Self::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            Self::Verify(msg) => write!(f, "verify failed: {msg}"),
            Self::Slot(msg) => write!(f, "slot error: {msg}"),
            Self::Resume(msg) => write!(f, "resume failed: {msg}"),
            Self::Unsupported(msg) => write!(f, "unsupported: {msg}"),
        }
    }
}

impl std::error::Error for FileVaultError {}

impl From<std::io::Error> for FileVaultError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for FileVaultError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VaultMode {
    Classic = 1,
}

impl VaultMode {
    fn from_u8(value: u8) -> Result<Self, FileVaultError> {
        match value {
            1 => Ok(Self::Classic),
            other => Err(FileVaultError::InvalidFormat(format!(
                "unsupported vault mode: {other}"
            ))),
        }
    }

    fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PayloadAlgorithm {
    Aes256Gcm = 1,
    XChaCha20Poly1305 = 2,
}

impl PayloadAlgorithm {
    pub fn from_cli(value: &str) -> Result<Self, FileVaultError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "aes256-gcm" | "aes-gcm" => Ok(Self::Aes256Gcm),
            "xchacha20-poly1305" | "xchacha20" | "xchacha" => Ok(Self::XChaCha20Poly1305),
            other => Err(FileVaultError::InvalidArgument(format!(
                "unsupported payload algorithm: {other}"
            ))),
        }
    }

    fn from_u8(value: u8) -> Result<Self, FileVaultError> {
        match value {
            1 => Ok(Self::Aes256Gcm),
            2 => Ok(Self::XChaCha20Poly1305),
            other => Err(FileVaultError::InvalidFormat(format!(
                "unsupported payload algorithm id: {other}"
            ))),
        }
    }

    fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aes256Gcm => "aes256-gcm",
            Self::XChaCha20Poly1305 => "xchacha20-poly1305",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HeaderMacAlgorithm {
    HmacSha256 = 1,
}

impl HeaderMacAlgorithm {
    fn from_u8(value: u8) -> Result<Self, FileVaultError> {
        match value {
            1 => Ok(Self::HmacSha256),
            other => Err(FileVaultError::InvalidFormat(format!(
                "unsupported header mac algorithm id: {other}"
            ))),
        }
    }

    fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum KdfKind {
    Argon2id = 1,
    Pbkdf2Sha256 = 2,
}

impl KdfKind {
    pub fn from_cli(value: &str) -> Result<Self, FileVaultError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "argon2id" => Ok(Self::Argon2id),
            "pbkdf2-sha256" | "pbkdf2" => Ok(Self::Pbkdf2Sha256),
            other => Err(FileVaultError::InvalidArgument(format!(
                "unsupported kdf: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SlotKind {
    Password,
    Keyfile,
    RecoveryKey,
    Dpapi,
}

impl SlotKind {
    pub fn from_cli(value: &str) -> Result<Self, FileVaultError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "password" => Ok(Self::Password),
            "keyfile" => Ok(Self::Keyfile),
            "recovery-key" | "recovery_key" | "recovery" => Ok(Self::RecoveryKey),
            "dpapi" => Ok(Self::Dpapi),
            other => Err(FileVaultError::InvalidArgument(format!(
                "unsupported slot kind: {other}"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Keyfile => "keyfile",
            Self::RecoveryKey => "recovery-key",
            Self::Dpapi => "dpapi",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KdfConfig {
    pub kind: KdfKind,
    pub salt_b64: String,
    pub mem_cost_kib: u32,
    pub time_cost: u32,
    pub parallelism: u32,
    pub rounds: u32,
}

impl KdfConfig {
    fn new(kind: KdfKind, salt: &[u8]) -> Self {
        match kind {
            KdfKind::Argon2id => Self {
                kind,
                salt_b64: b64(salt),
                mem_cost_kib: 64 * 1024,
                time_cost: 3,
                parallelism: 1,
                rounds: 0,
            },
            KdfKind::Pbkdf2Sha256 => Self {
                kind,
                salt_b64: b64(salt),
                mem_cost_kib: 0,
                time_cost: 0,
                parallelism: 0,
                rounds: 900_000,
            },
        }
    }

    fn salt(&self) -> Result<Vec<u8>, FileVaultError> {
        b64_decode(&self.salt_b64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilenameInfo {
    nonce_b64: String,
    ciphertext_b64: String,
    aad_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotRecord {
    kind: SlotKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    kdf: Option<KdfConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce_b64: Option<String>,
    wrapped_key_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VariableHeader {
    slots: Vec<SlotRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<FilenameInfo>,
}

#[derive(Debug, Clone)]
struct FixedHeader {
    flags: u16,
    mode: VaultMode,
    payload_algorithm: PayloadAlgorithm,
    header_mac_algorithm: HeaderMacAlgorithm,
    chunk_size: u32,
    variable_header_len: u32,
    plaintext_len: u64,
    frame_count: u32,
    created_unix: u64,
    nonce_seed: [u8; 16],
}

#[derive(Debug, Clone)]
struct FrameHeader {
    sequence: u32,
    plaintext_len: u32,
    ciphertext_len: u32,
    ciphertext_crc32: u32,
}

#[derive(Debug, Clone)]
struct CommitFooter {
    payload_digest: [u8; 32],
    header_mac: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum JournalState {
    Writing,
    ReadyToCommit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u16,
    state: JournalState,
    input_path: String,
    output_path: String,
    temp_path: String,
    fixed_header_b64: String,
    variable_header_b64: String,
    file_key_b64: String,
    header_mac_offset: u64,
    first_frame_offset: u64,
    frames_written: u32,
    plaintext_len: u64,
}

#[derive(Debug, Clone)]
struct ParsedVault {
    fixed_header: FixedHeader,
    fixed_header_bytes: [u8; FIXED_HEADER_LEN],
    variable_header: VariableHeader,
    variable_header_bytes: Vec<u8>,
    header_mac: [u8; HEADER_MAC_LEN],
    first_frame_offset: u64,
    second_frame_offset: Option<u64>,
    frame_span: Option<u64>,
    footer_offset: Option<u64>,
    footer: Option<CommitFooter>,
    file_len: u64,
}

#[derive(Debug, Clone, Default)]
pub struct UnlockOptions {
    pub password: Option<String>,
    pub keyfile: Option<PathBuf>,
    pub recovery_key: Option<String>,
    pub dpapi: bool,
}

#[derive(Debug, Clone)]
pub struct EncryptOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub password: Option<String>,
    pub keyfile: Option<PathBuf>,
    pub recovery_key: Option<String>,
    pub emit_recovery_key: Option<PathBuf>,
    pub dpapi: bool,
    pub payload_algorithm: PayloadAlgorithm,
    pub kdf: KdfKind,
    pub chunk_size: u32,
}

#[derive(Debug, Clone)]
pub struct DecryptOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub unlock: UnlockOptions,
}

#[derive(Debug, Clone)]
pub struct VerifyOptions {
    pub input: PathBuf,
    pub unlock: UnlockOptions,
}

#[derive(Debug, Clone)]
pub struct RewrapOptions {
    pub path: PathBuf,
    pub unlock: UnlockOptions,
    pub add_password: Option<String>,
    pub add_keyfile: Option<PathBuf>,
    pub add_recovery_key: Option<String>,
    pub emit_recovery_key: Option<PathBuf>,
    pub add_dpapi: bool,
    pub remove_slots: Vec<SlotKind>,
    pub kdf: KdfKind,
}

#[derive(Debug, Clone)]
pub struct RecoverKeyOptions {
    pub path: PathBuf,
    pub unlock: UnlockOptions,
    pub output: PathBuf,
}

pub fn encrypt_file(options: &EncryptOptions) -> Result<Value, FileVaultError> {
    let mut timing = StepTiming::new("encrypt");
    validate_encrypt_options(options)?;
    timing.record("validate options");
    let temp_path = temp_path_for(&options.output);
    let journal_path = journal_path_for(&options.output);
    cleanup_sidecars(&options.output)?;
    timing.record("prepare sidecars");

    let plaintext_len = fs::metadata(&options.input)?.len();
    let mut file_key = random_bytes(32)?;
    let nonce_seed_vec = random_bytes(16)?;
    let mut nonce_seed = [0u8; 16];
    nonce_seed.copy_from_slice(&nonce_seed_vec);
    let variable_header = build_variable_header(
        &options.input,
        &file_key,
        options.password.as_deref(),
        options.keyfile.as_deref(),
        options.recovery_key.as_deref(),
        options.emit_recovery_key.as_deref(),
        options.dpapi,
        options.kdf,
    )?;
    let variable_header_bytes = serde_json::to_vec(&variable_header)?;
    let frame_count = compute_frame_count(plaintext_len, options.chunk_size);
    let fixed_header = FixedHeader {
        flags: if variable_header.filename.is_some() {
            FLAG_FILENAME_ENCRYPTED
        } else {
            0
        },
        mode: VaultMode::Classic,
        payload_algorithm: options.payload_algorithm,
        header_mac_algorithm: HeaderMacAlgorithm::HmacSha256,
        chunk_size: options.chunk_size,
        variable_header_len: variable_header_bytes.len() as u32,
        plaintext_len,
        frame_count,
        created_unix: now_unix_secs(),
        nonce_seed,
    };
    let fixed_header_bytes = encode_fixed_header(&fixed_header);
    let header_mac_offset = (FIXED_HEADER_LEN + variable_header_bytes.len()) as u64;
    let first_frame_offset = header_mac_offset + HEADER_MAC_LEN as u64;
    timing.record("build headers");

    let journal = Journal {
        version: FORMAT_VERSION,
        state: JournalState::Writing,
        input_path: options.input.to_string_lossy().to_string(),
        output_path: options.output.to_string_lossy().to_string(),
        temp_path: temp_path.to_string_lossy().to_string(),
        fixed_header_b64: b64(&fixed_header_bytes),
        variable_header_b64: b64(&variable_header_bytes),
        file_key_b64: b64(&file_key),
        header_mac_offset,
        first_frame_offset,
        frames_written: 0,
        plaintext_len,
    };

    let mut input = File::open(&options.input)?;
    let mut temp = File::create(&temp_path)?;
    temp.write_all(&fixed_header_bytes)?;
    temp.write_all(&variable_header_bytes)?;
    temp.write_all(&[0u8; HEADER_MAC_LEN])?;
    write_journal(&journal_path, &journal)?;
    timing.record("open/write header");

    let fail_after_frames = std::env::var("XUN_FILEVAULT_FAIL_AFTER_FRAMES")
        .ok()
        .and_then(|value| value.parse::<u32>().ok());
    let mut buffer = vec![0u8; options.chunk_size as usize];
    let mut journal = journal;
    let mut sequence = 0u32;
    let mut payload_hasher = Sha256::new();
    let worker_count = filevault_workers();
    let inflight = filevault_inflight(worker_count);
    if worker_count <= 1 {
        loop {
            let read = input.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            let ciphertext = encrypt_frame(
                fixed_header.payload_algorithm,
                &file_key,
                &fixed_header.nonce_seed,
                sequence,
                &buffer[..read],
            )?;
            let frame_header = FrameHeader {
                sequence,
                plaintext_len: read as u32,
                ciphertext_len: ciphertext.len() as u32,
                ciphertext_crc32: crc32(&ciphertext),
            };
            let frame_header_bytes = encode_frame_header(&frame_header);
            temp.write_all(&frame_header_bytes)?;
            temp.write_all(&ciphertext)?;
            payload_hasher.update(&frame_header_bytes);
            payload_hasher.update(&ciphertext);
            journal.frames_written = sequence + 1;
            write_journal(&journal_path, &journal)?;
            if fail_after_frames == Some(journal.frames_written) {
                return Err(FileVaultError::Resume(
                    "simulated crash after streaming frames; use `vault resume`".to_string(),
                ));
            }
            sequence += 1;
        }
    } else {
        let (job_tx, job_rx) = mpsc::sync_channel::<EncJob>(inflight);
        let job_rx = Arc::new(Mutex::new(job_rx));
        let (res_tx, res_rx) = mpsc::channel::<Result<EncFrame, FileVaultError>>();
        let mut handles = Vec::new();
        for _ in 0..worker_count {
            let job_rx = Arc::clone(&job_rx);
            let res_tx = res_tx.clone();
            let mut worker_key = file_key.clone();
            let nonce_seed = fixed_header.nonce_seed;
            let algorithm = fixed_header.payload_algorithm;
            handles.push(thread::spawn(move || {
                loop {
                    let job = {
                        let guard = job_rx.lock().unwrap();
                        guard.recv()
                    };
                    let job = match job {
                        Ok(job) => job,
                        Err(_) => break,
                    };
                    let result = encrypt_frame(
                        algorithm,
                        &worker_key,
                        &nonce_seed,
                        job.sequence,
                        &job.plaintext,
                    )
                    .map(|ciphertext| {
                        let frame_header = FrameHeader {
                            sequence: job.sequence,
                            plaintext_len: job.plaintext.len() as u32,
                            ciphertext_len: ciphertext.len() as u32,
                            ciphertext_crc32: crc32(&ciphertext),
                        };
                        let header_bytes = encode_frame_header(&frame_header);
                        EncFrame {
                            sequence: job.sequence,
                            header_bytes,
                            ciphertext,
                        }
                    });
                    if res_tx.send(result).is_err() {
                        break;
                    }
                }
                worker_key.zeroize();
            }));
        }
        drop(res_tx);

        let mut pending: BTreeMap<u32, EncFrame> = BTreeMap::new();
        let mut next_write = 0u32;
        let mut in_flight = 0usize;
        let mut send_seq = 0u32;
        loop {
            let read = input.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            let plaintext = buffer[..read].to_vec();
            job_tx
                .send(EncJob {
                    sequence: send_seq,
                    plaintext,
                })
                .map_err(|_| FileVaultError::Crypto("worker channel closed"))?;
            in_flight += 1;
            send_seq += 1;
            if in_flight >= inflight {
                let result = res_rx
                    .recv()
                    .map_err(|_| FileVaultError::Crypto("worker channel closed"))?;
                in_flight -= 1;
                match result {
                    Ok(frame) => {
                        pending.insert(frame.sequence, frame);
                    }
                    Err(err) => {
                        drop(job_tx);
                        return Err(err);
                    }
                }
                while let Some(frame) = pending.remove(&next_write) {
                    temp.write_all(&frame.header_bytes)?;
                    temp.write_all(&frame.ciphertext)?;
                    payload_hasher.update(&frame.header_bytes);
                    payload_hasher.update(&frame.ciphertext);
                    journal.frames_written = next_write + 1;
                    write_journal(&journal_path, &journal)?;
                    if fail_after_frames == Some(journal.frames_written) {
                        drop(job_tx);
                        return Err(FileVaultError::Resume(
                            "simulated crash after streaming frames; use `vault resume`"
                                .to_string(),
                        ));
                    }
                    next_write += 1;
                }
            }
        }
        drop(job_tx);
        while in_flight > 0 {
            let result = res_rx
                .recv()
                .map_err(|_| FileVaultError::Crypto("worker channel closed"))?;
            in_flight -= 1;
            match result {
                Ok(frame) => {
                    pending.insert(frame.sequence, frame);
                }
                Err(err) => return Err(err),
            }
            while let Some(frame) = pending.remove(&next_write) {
                temp.write_all(&frame.header_bytes)?;
                temp.write_all(&frame.ciphertext)?;
                payload_hasher.update(&frame.header_bytes);
                payload_hasher.update(&frame.ciphertext);
                journal.frames_written = next_write + 1;
                write_journal(&journal_path, &journal)?;
                if fail_after_frames == Some(journal.frames_written) {
                    return Err(FileVaultError::Resume(
                        "simulated crash after streaming frames; use `vault resume`".to_string(),
                    ));
                }
                next_write += 1;
            }
        }
        for handle in handles {
            let _ = handle.join();
        }
    }
    timing.record("stream frames");

    let payload_digest: [u8; 32] = payload_hasher.finalize().into();
    timing.record("payload digest");
    let header_mac =
        compute_header_mac(&payload_digest, &fixed_header_bytes, &variable_header_bytes)?;
    temp.seek(SeekFrom::Start(header_mac_offset))?;
    temp.write_all(&header_mac)?;
    let footer = CommitFooter {
        payload_digest,
        header_mac,
    };
    temp.seek(SeekFrom::End(0))?;
    temp.write_all(&encode_footer(&footer))?;
    temp.flush()?;
    timing.record("footer write");

    journal.state = JournalState::ReadyToCommit;
    write_journal(&journal_path, &journal)?;
    commit_temp_into_place(&temp_path, &options.output)?;
    fs::remove_file(&journal_path)?;
    file_key.zeroize();
    timing.record("commit rename");
    timing.finish();

    Ok(json!({
        "status": "ok",
        "output": options.output.to_string_lossy(),
        "frames_written": fixed_header.frame_count,
        "chunk_size": options.chunk_size,
        "algorithm": fixed_header.payload_algorithm.as_str(),
    }))
}

pub fn decrypt_file(options: &DecryptOptions) -> Result<Value, FileVaultError> {
    let mut timing = StepTiming::new("decrypt");
    let parsed = parse_vault(&options.input)?;
    timing.record("parse");
    let footer = parsed
        .footer
        .clone()
        .ok_or_else(|| FileVaultError::Verify("commit footer missing".to_string()))?;
    let mut file_key = unlock_file_key(&parsed.variable_header, &options.unlock)?;
    timing.record("unlock");

    let temp_output = temp_path_for(&options.output);
    if temp_output.exists() {
        fs::remove_file(&temp_output)?;
    }
    let mut source = File::open(&options.input)?;
    let mut output = File::create(&temp_output)?;
    source.seek(SeekFrom::Start(parsed.first_frame_offset))?;
    timing.record("open output");
    let mut payload_hasher = Sha256::new();
    let worker_count = filevault_workers();
    let inflight = filevault_inflight(worker_count);
    if worker_count <= 1 {
        let mut ciphertext = Vec::new();
        for expected_seq in 0..parsed.fixed_header.frame_count {
            let header = read_frame_header(&mut source)?;
            if header.sequence != expected_seq {
                return Err(FileVaultError::Verify(format!(
                    "frame sequence mismatch at {expected_seq}"
                )));
            }
            let needed = header.ciphertext_len as usize;
            if ciphertext.len() != needed {
                ciphertext.resize(needed, 0);
            }
            source.read_exact(&mut ciphertext)?;
            if crc32(&ciphertext) != header.ciphertext_crc32 {
                return Err(FileVaultError::Verify(format!(
                    "frame crc mismatch at {expected_seq}"
                )));
            }
            let frame_header_bytes = encode_frame_header(&header);
            payload_hasher.update(&frame_header_bytes);
            payload_hasher.update(&ciphertext);
            let plaintext = decrypt_frame(
                parsed.fixed_header.payload_algorithm,
                &file_key,
                &parsed.fixed_header.nonce_seed,
                expected_seq,
                &ciphertext,
                header.plaintext_len,
            )?;
            output.write_all(&plaintext)?;
        }
    } else {
        let (job_tx, job_rx) = mpsc::sync_channel::<DecJob>(inflight);
        let job_rx = Arc::new(Mutex::new(job_rx));
        let (res_tx, res_rx) = mpsc::channel::<Result<DecFrame, FileVaultError>>();
        let mut handles = Vec::new();
        for _ in 0..worker_count {
            let job_rx = Arc::clone(&job_rx);
            let res_tx = res_tx.clone();
            let mut worker_key = file_key.clone();
            let nonce_seed = parsed.fixed_header.nonce_seed;
            let algorithm = parsed.fixed_header.payload_algorithm;
            handles.push(thread::spawn(move || {
                loop {
                    let job = {
                        let guard = job_rx.lock().unwrap();
                        guard.recv()
                    };
                    let job = match job {
                        Ok(job) => job,
                        Err(_) => break,
                    };
                    let result = decrypt_frame(
                        algorithm,
                        &worker_key,
                        &nonce_seed,
                        job.sequence,
                        &job.ciphertext,
                        job.plaintext_len,
                    )
                    .map(|plaintext| DecFrame {
                        sequence: job.sequence,
                        plaintext,
                    });
                    if res_tx.send(result).is_err() {
                        break;
                    }
                }
                worker_key.zeroize();
            }));
        }
        drop(res_tx);

        let mut pending: BTreeMap<u32, DecFrame> = BTreeMap::new();
        let mut next_write = 0u32;
        let mut in_flight = 0usize;
        for expected_seq in 0..parsed.fixed_header.frame_count {
            let header = read_frame_header(&mut source)?;
            if header.sequence != expected_seq {
                return Err(FileVaultError::Verify(format!(
                    "frame sequence mismatch at {expected_seq}"
                )));
            }
            let mut ciphertext = vec![0u8; header.ciphertext_len as usize];
            source.read_exact(&mut ciphertext)?;
            if crc32(&ciphertext) != header.ciphertext_crc32 {
                return Err(FileVaultError::Verify(format!(
                    "frame crc mismatch at {expected_seq}"
                )));
            }
            let frame_header_bytes = encode_frame_header(&header);
            payload_hasher.update(&frame_header_bytes);
            payload_hasher.update(&ciphertext);
            job_tx
                .send(DecJob {
                    sequence: expected_seq,
                    plaintext_len: header.plaintext_len,
                    ciphertext,
                })
                .map_err(|_| FileVaultError::Crypto("worker channel closed"))?;
            in_flight += 1;
            if in_flight >= inflight {
                let result = res_rx
                    .recv()
                    .map_err(|_| FileVaultError::Crypto("worker channel closed"))?;
                in_flight -= 1;
                match result {
                    Ok(frame) => {
                        pending.insert(frame.sequence, frame);
                    }
                    Err(err) => {
                        drop(job_tx);
                        return Err(err);
                    }
                }
                while let Some(frame) = pending.remove(&next_write) {
                    output.write_all(&frame.plaintext)?;
                    next_write += 1;
                }
            }
        }
        drop(job_tx);
        while in_flight > 0 {
            let result = res_rx
                .recv()
                .map_err(|_| FileVaultError::Crypto("worker channel closed"))?;
            in_flight -= 1;
            match result {
                Ok(frame) => {
                    pending.insert(frame.sequence, frame);
                }
                Err(err) => return Err(err),
            }
            while let Some(frame) = pending.remove(&next_write) {
                output.write_all(&frame.plaintext)?;
                next_write += 1;
            }
        }
        for handle in handles {
            let _ = handle.join();
        }
    }
    output.flush()?;
    timing.record("stream frames");
    let payload_digest: [u8; 32] = payload_hasher.finalize().into();
    if let Some(footer_offset) = parsed.footer_offset {
        let current = source.stream_position()?;
        if current != footer_offset {
            let _ = fs::remove_file(&temp_output);
            return Err(FileVaultError::Verify(
                "frame layout length mismatch".to_string(),
            ));
        }
    }
    if payload_digest != footer.payload_digest {
        let _ = fs::remove_file(&temp_output);
        return Err(FileVaultError::Verify("frame digest mismatch".to_string()));
    }
    let expected_header_mac = compute_header_mac(
        &payload_digest,
        &parsed.fixed_header_bytes,
        &parsed.variable_header_bytes,
    )?;
    if expected_header_mac != parsed.header_mac || expected_header_mac != footer.header_mac {
        let _ = fs::remove_file(&temp_output);
        return Err(FileVaultError::Verify("header mac mismatch".to_string()));
    }
    timing.record("verify footer");
    commit_temp_into_place(&temp_output, &options.output)?;
    file_key.zeroize();
    timing.record("commit rename");
    timing.finish();

    Ok(json!({
        "status": "ok",
        "output": options.output.to_string_lossy(),
        "payload_digest": hex_string(&footer.payload_digest),
    }))
}

pub fn inspect_file(path: &Path) -> Result<Value, FileVaultError> {
    let parsed = parse_vault(path)?;
    let footer = parsed.footer.as_ref();
    Ok(json!({
        "status": if footer.is_some() { "ok" } else { "incomplete" },
        "path": path.to_string_lossy(),
        "header": {
            "version": FORMAT_VERSION,
            "mode": "classic",
            "flags": parsed.fixed_header.flags,
            "payload_algorithm": parsed.fixed_header.payload_algorithm.as_str(),
            "header_mac_algorithm": "hmac-sha256",
            "chunk_size": parsed.fixed_header.chunk_size,
            "plaintext_len": parsed.fixed_header.plaintext_len,
            "frame_count": parsed.fixed_header.frame_count,
            "created_unix": parsed.fixed_header.created_unix,
        },
        "slots": parsed.variable_header.slots.iter().map(|slot| json!({
            "kind": slot.kind.as_str(),
            "has_kdf": slot.kdf.is_some(),
        })).collect::<Vec<_>>(),
        "layout": {
            "first_frame_offset": parsed.first_frame_offset,
            "second_frame_offset": parsed.second_frame_offset,
            "frame_span": parsed.frame_span,
            "footer_offset": parsed.footer_offset,
            "file_len": parsed.file_len,
        },
        "footer": {
            "present": footer.is_some(),
            "payload_digest": footer.map(|f| hex_string(&f.payload_digest)),
            "header_mac": footer.map(|f| hex_string(&f.header_mac)),
        }
    }))
}

pub fn verify_file(options: &VerifyOptions) -> Result<Value, FileVaultError> {
    let parsed = parse_vault(&options.input)?;
    let report = match verify_parsed(&options.input, &parsed, true) {
        Ok(mut value) => {
            if unlock_supplied(&options.unlock) {
                let mut file_key = unlock_file_key(&parsed.variable_header, &options.unlock)?;
                verify_frame_decryption(&options.input, &parsed, &file_key)?;
                file_key.zeroize();
                value["decryption"] = json!({"valid": true});
            }
            value
        }
        Err(err) => return Err(err),
    };
    Ok(report)
}

pub fn resume_file(path: &Path) -> Result<Value, FileVaultError> {
    let journal_path = journal_path_for(path);
    let journal = read_journal(&journal_path)?;
    let fixed_header_bytes = vec_from_b64(&journal.fixed_header_b64)?;
    if fixed_header_bytes.len() != FIXED_HEADER_LEN {
        return Err(FileVaultError::Resume(
            "fixed header length mismatch in journal".to_string(),
        ));
    }
    let fixed_header = decode_fixed_header(
        fixed_header_bytes
            .clone()
            .try_into()
            .map_err(|_| FileVaultError::Resume("invalid fixed header bytes".to_string()))?,
    )?;
    let variable_header_bytes = vec_from_b64(&journal.variable_header_b64)?;
    let mut file_key = vec_from_b64(&journal.file_key_b64)?;
    let input_path = PathBuf::from(journal.input_path);
    let temp_path = PathBuf::from(journal.temp_path);
    let output_path = PathBuf::from(journal.output_path);

    let mut input = File::open(&input_path)?;
    input.seek(SeekFrom::Start(
        journal.frames_written as u64 * fixed_header.chunk_size as u64,
    ))?;
    let mut temp = File::options().read(true).write(true).open(&temp_path)?;
    temp.seek(SeekFrom::End(0))?;
    let mut buffer = vec![0u8; fixed_header.chunk_size as usize];
    let mut sequence = journal.frames_written;
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let ciphertext = encrypt_frame(
            fixed_header.payload_algorithm,
            &file_key,
            &fixed_header.nonce_seed,
            sequence,
            &buffer[..read],
        )?;
        let frame_header = FrameHeader {
            sequence,
            plaintext_len: read as u32,
            ciphertext_len: ciphertext.len() as u32,
            ciphertext_crc32: crc32(&ciphertext),
        };
        temp.write_all(&encode_frame_header(&frame_header))?;
        temp.write_all(&ciphertext)?;
        sequence += 1;
    }

    let payload_digest = compute_payload_digest(
        &temp_path,
        journal.first_frame_offset,
        fixed_header.frame_count,
        None,
    )?;
    let header_mac = compute_header_mac(
        &payload_digest,
        fixed_header_bytes.as_slice().try_into().unwrap(),
        &variable_header_bytes,
    )?;
    temp.seek(SeekFrom::Start(journal.header_mac_offset))?;
    temp.write_all(&header_mac)?;
    temp.seek(SeekFrom::End(0))?;
    temp.write_all(&encode_footer(&CommitFooter {
        payload_digest,
        header_mac,
    }))?;
    temp.flush()?;
    commit_temp_into_place(&temp_path, &output_path)?;
    fs::remove_file(journal_path)?;
    file_key.zeroize();

    Ok(json!({
        "status": "ok",
        "output": output_path.to_string_lossy(),
        "resumed_from_frame": journal.frames_written,
    }))
}

pub fn cleanup_artifacts(path: &Path) -> Result<Value, FileVaultError> {
    let temp_path = temp_path_for(path);
    let journal_path = journal_path_for(path);
    let mut removed = Vec::new();
    if temp_path.exists() {
        fs::remove_file(&temp_path)?;
        removed.push(temp_path.to_string_lossy().to_string());
    }
    if journal_path.exists() {
        fs::remove_file(&journal_path)?;
        removed.push(journal_path.to_string_lossy().to_string());
    }
    Ok(json!({"status": "ok", "removed": removed}))
}

pub fn rewrap_file(options: &RewrapOptions) -> Result<Value, FileVaultError> {
    let parsed = parse_vault(&options.path)?;
    let footer = parsed
        .footer
        .clone()
        .ok_or_else(|| FileVaultError::Verify("commit footer missing".to_string()))?;
    verify_parsed(&options.path, &parsed, false)?;
    let mut file_key = unlock_file_key(&parsed.variable_header, &options.unlock)?;

    let mut slots = parsed.variable_header.slots.clone();
    if !options.remove_slots.is_empty() {
        slots.retain(|slot| !options.remove_slots.contains(&slot.kind));
    }
    if let Some(password) = options.add_password.as_deref() {
        replace_slot(
            &mut slots,
            build_password_slot(password, &file_key, options.kdf)?,
        );
    }
    if let Some(keyfile) = options.add_keyfile.as_deref() {
        replace_slot(
            &mut slots,
            build_keyfile_slot(keyfile, &file_key, options.kdf)?,
        );
    }
    if let Some(recovery_key) = options.add_recovery_key.as_deref() {
        replace_slot(&mut slots, build_recovery_slot(recovery_key, &file_key)?);
    }
    if let Some(output_path) = options.emit_recovery_key.as_deref() {
        let recovery_key = generate_recovery_key()?;
        fs::write(output_path, format!("{recovery_key}\n"))?;
        replace_slot(&mut slots, build_recovery_slot(&recovery_key, &file_key)?);
    }
    if options.add_dpapi {
        replace_slot(&mut slots, build_dpapi_slot(&file_key)?);
    }
    if slots.is_empty() {
        return Err(FileVaultError::InvalidArgument(
            "rewrap would remove every legal slot".to_string(),
        ));
    }

    let variable_header = VariableHeader {
        slots,
        filename: parsed.variable_header.filename.clone(),
    };
    let variable_header_bytes = serde_json::to_vec(&variable_header)?;
    let mut fixed_header = parsed.fixed_header.clone();
    fixed_header.variable_header_len = variable_header_bytes.len() as u32;
    let fixed_header_bytes = encode_fixed_header(&fixed_header);
    let header_mac = compute_header_mac(
        &footer.payload_digest,
        &fixed_header_bytes,
        &variable_header_bytes,
    )?;
    let new_footer = CommitFooter {
        payload_digest: footer.payload_digest,
        header_mac,
    };

    let temp_path = temp_path_for(&options.path);
    if temp_path.exists() {
        fs::remove_file(&temp_path)?;
    }
    let mut source = File::open(&options.path)?;
    let mut temp = File::create(&temp_path)?;
    temp.write_all(&fixed_header_bytes)?;
    temp.write_all(&variable_header_bytes)?;
    temp.write_all(&header_mac)?;

    let payload_len = parsed
        .footer_offset
        .ok_or_else(|| FileVaultError::Verify("commit footer missing".to_string()))?
        .saturating_sub(parsed.first_frame_offset);
    source.seek(SeekFrom::Start(parsed.first_frame_offset))?;
    copy_exact_bytes(&mut source, &mut temp, payload_len)?;
    temp.write_all(&encode_footer(&new_footer))?;
    temp.flush()?;
    replace_existing_file(&temp_path, &options.path)?;
    file_key.zeroize();

    Ok(json!({
        "status": "ok",
        "payload_digest": hex_string(&footer.payload_digest),
        "slot_kinds": variable_header.slots.iter().map(|slot| slot.kind.as_str()).collect::<Vec<_>>(),
    }))
}

pub fn recover_key_file(options: &RecoverKeyOptions) -> Result<Value, FileVaultError> {
    let recovery_key = generate_recovery_key()?;
    fs::write(&options.output, format!("{recovery_key}\n"))?;
    let rewrap = RewrapOptions {
        path: options.path.clone(),
        unlock: options.unlock.clone(),
        add_password: None,
        add_keyfile: None,
        add_recovery_key: Some(recovery_key.clone()),
        emit_recovery_key: None,
        add_dpapi: false,
        remove_slots: Vec::new(),
        kdf: KdfKind::Argon2id,
    };
    let mut value = rewrap_file(&rewrap)?;
    value["recovery_key_output"] = json!(options.output.to_string_lossy().to_string());
    Ok(value)
}

fn validate_encrypt_options(options: &EncryptOptions) -> Result<(), FileVaultError> {
    if !options.input.exists() {
        return Err(FileVaultError::InvalidArgument(format!(
            "input not found: {}",
            options.input.display()
        )));
    }
    if options.chunk_size == 0 {
        return Err(FileVaultError::InvalidArgument(
            "chunk size must be greater than zero".to_string(),
        ));
    }
    if options.password.is_none()
        && options.keyfile.is_none()
        && options.recovery_key.is_none()
        && options.emit_recovery_key.is_none()
        && !options.dpapi
    {
        return Err(FileVaultError::InvalidArgument(
            "at least one slot is required".to_string(),
        ));
    }
    Ok(())
}

fn build_variable_header(
    input: &Path,
    file_key: &[u8],
    password: Option<&str>,
    keyfile: Option<&Path>,
    recovery_key: Option<&str>,
    emit_recovery_key: Option<&Path>,
    dpapi: bool,
    kdf: KdfKind,
) -> Result<VariableHeader, FileVaultError> {
    let mut slots = Vec::new();
    if let Some(password) = password {
        slots.push(build_password_slot(password, file_key, kdf)?);
    }
    if let Some(keyfile) = keyfile {
        slots.push(build_keyfile_slot(keyfile, file_key, kdf)?);
    }
    if let Some(recovery_key) = recovery_key {
        slots.push(build_recovery_slot(recovery_key, file_key)?);
    }
    if let Some(output_path) = emit_recovery_key {
        let recovery_key = generate_recovery_key()?;
        fs::write(output_path, format!("{recovery_key}\n"))?;
        slots.push(build_recovery_slot(&recovery_key, file_key)?);
    }
    if dpapi {
        slots.push(build_dpapi_slot(file_key)?);
    }
    let filename = encrypt_filename_metadata(input, file_key)?;
    Ok(VariableHeader { slots, filename })
}

fn replace_slot(slots: &mut Vec<SlotRecord>, new_slot: SlotRecord) {
    slots.retain(|slot| slot.kind != new_slot.kind);
    slots.push(new_slot);
}

fn parse_vault(path: &Path) -> Result<ParsedVault, FileVaultError> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    if file_len < (FIXED_HEADER_LEN + HEADER_MAC_LEN) as u64 {
        return Err(FileVaultError::InvalidFormat(
            "file too small for v13 header".to_string(),
        ));
    }

    let mut fixed_header_bytes = [0u8; FIXED_HEADER_LEN];
    file.read_exact(&mut fixed_header_bytes)?;
    let fixed_header = decode_fixed_header(fixed_header_bytes)?;
    let mut variable_header_bytes = vec![0u8; fixed_header.variable_header_len as usize];
    file.read_exact(&mut variable_header_bytes)?;
    let variable_header: VariableHeader = serde_json::from_slice(&variable_header_bytes)?;
    let mut header_mac = [0u8; HEADER_MAC_LEN];
    file.read_exact(&mut header_mac)?;
    let first_frame_offset =
        (FIXED_HEADER_LEN + variable_header_bytes.len() + HEADER_MAC_LEN) as u64;

    let mut footer = None;
    let mut footer_offset = None;
    if file_len >= first_frame_offset + FOOTER_LEN as u64 {
        let candidate_offset = file_len - FOOTER_LEN as u64;
        file.seek(SeekFrom::Start(candidate_offset))?;
        let mut footer_bytes = [0u8; FOOTER_LEN];
        file.read_exact(&mut footer_bytes)?;
        if &footer_bytes[..8] == COMMIT_MAGIC {
            footer = Some(decode_footer(footer_bytes)?);
            footer_offset = Some(candidate_offset);
        }
    }

    let (second_frame_offset, frame_span) = if fixed_header.frame_count >= 2 {
        file.seek(SeekFrom::Start(first_frame_offset))?;
        let first_header = read_frame_header(&mut file)?;
        let span = FRAME_HEADER_LEN as u64 + first_header.ciphertext_len as u64;
        (Some(first_frame_offset + span), Some(span))
    } else if fixed_header.frame_count == 1 {
        file.seek(SeekFrom::Start(first_frame_offset))?;
        let first_header = read_frame_header(&mut file)?;
        let span = FRAME_HEADER_LEN as u64 + first_header.ciphertext_len as u64;
        (None, Some(span))
    } else {
        (None, None)
    };

    Ok(ParsedVault {
        fixed_header,
        fixed_header_bytes,
        variable_header,
        variable_header_bytes,
        header_mac,
        first_frame_offset,
        second_frame_offset,
        frame_span,
        footer_offset,
        footer,
        file_len,
    })
}

fn verify_parsed(
    path: &Path,
    parsed: &ParsedVault,
    json_only: bool,
) -> Result<Value, FileVaultError> {
    let footer = match parsed.footer.as_ref() {
        Some(footer) => footer,
        None => {
            let value = json!({
                "status": "incomplete",
                "header": {"valid": false, "reason": "header mac unavailable without footer"},
                "payload": {"valid": false, "reason": "payload digest unavailable without footer"},
                "footer": {"present": false}
            });
            return if json_only {
                Err(FileVaultError::Verify(value.to_string()))
            } else {
                Ok(value)
            };
        }
    };

    let payload_digest = match compute_payload_digest(
        path,
        parsed.first_frame_offset,
        parsed.fixed_header.frame_count,
        parsed.footer_offset,
    ) {
        Ok(digest) => digest,
        Err(err) => {
            let value = json!({
                "status": "corrupt",
                "header": {"valid": false, "reason": err.to_string()},
                "payload": {"valid": false, "reason": err.to_string()},
                "footer": {"present": true}
            });
            return Err(FileVaultError::Verify(value.to_string()));
        }
    };
    if payload_digest != footer.payload_digest {
        let value = json!({
            "status": "corrupt",
            "header": {"valid": false, "reason": "header mac not trusted because payload digest changed"},
            "payload": {"valid": false, "reason": "frame digest mismatch"},
            "footer": {"present": true}
        });
        return Err(FileVaultError::Verify(value.to_string()));
    }

    let expected_header_mac = compute_header_mac(
        &payload_digest,
        &parsed.fixed_header_bytes,
        &parsed.variable_header_bytes,
    )?;
    if expected_header_mac != parsed.header_mac || expected_header_mac != footer.header_mac {
        let value = json!({
            "status": "corrupt",
            "header": {"valid": false, "reason": "header mac mismatch"},
            "payload": {"valid": true},
            "footer": {"present": true}
        });
        return Err(FileVaultError::Verify(value.to_string()));
    }

    Ok(json!({
        "status": "ok",
        "header": {"valid": true},
        "payload": {"valid": true, "digest": hex_string(&payload_digest)},
        "footer": {"present": true}
    }))
}

fn verify_frame_decryption(
    path: &Path,
    parsed: &ParsedVault,
    file_key: &[u8],
) -> Result<(), FileVaultError> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(parsed.first_frame_offset))?;
    let mut ciphertext = Vec::new();
    for expected_seq in 0..parsed.fixed_header.frame_count {
        let header = read_frame_header(&mut file)?;
        let needed = header.ciphertext_len as usize;
        if ciphertext.len() != needed {
            ciphertext.resize(needed, 0);
        }
        file.read_exact(&mut ciphertext)?;
        let _ = decrypt_frame(
            parsed.fixed_header.payload_algorithm,
            file_key,
            &parsed.fixed_header.nonce_seed,
            expected_seq,
            &ciphertext,
            header.plaintext_len,
        )?;
    }
    Ok(())
}

fn build_password_slot(
    password: &str,
    file_key: &[u8],
    kdf_kind: KdfKind,
) -> Result<SlotRecord, FileVaultError> {
    let salt = random_bytes(16)?;
    let kdf = KdfConfig::new(kdf_kind, &salt);
    let wrap_key = derive_password_key(password.as_bytes(), &kdf)?;
    let (nonce, wrapped) = wrap_key_material(&wrap_key, file_key, SlotKind::Password)?;
    Ok(SlotRecord {
        kind: SlotKind::Password,
        kdf: Some(kdf),
        nonce_b64: Some(b64(&nonce)),
        wrapped_key_b64: b64(&wrapped),
    })
}

fn build_keyfile_slot(
    path: &Path,
    file_key: &[u8],
    kdf_kind: KdfKind,
) -> Result<SlotRecord, FileVaultError> {
    let keyfile_bytes = fs::read(path)?;
    let salt = random_bytes(16)?;
    let kdf = KdfConfig::new(kdf_kind, &salt);
    let wrap_key = derive_password_key(&keyfile_bytes, &kdf)?;
    let (nonce, wrapped) = wrap_key_material(&wrap_key, file_key, SlotKind::Keyfile)?;
    Ok(SlotRecord {
        kind: SlotKind::Keyfile,
        kdf: Some(kdf),
        nonce_b64: Some(b64(&nonce)),
        wrapped_key_b64: b64(&wrapped),
    })
}

fn build_recovery_slot(recovery_key: &str, file_key: &[u8]) -> Result<SlotRecord, FileVaultError> {
    let wrap_key = derive_recovery_key(recovery_key)?;
    let (nonce, wrapped) = wrap_key_material(&wrap_key, file_key, SlotKind::RecoveryKey)?;
    Ok(SlotRecord {
        kind: SlotKind::RecoveryKey,
        kdf: None,
        nonce_b64: Some(b64(&nonce)),
        wrapped_key_b64: b64(&wrapped),
    })
}

fn build_dpapi_slot(file_key: &[u8]) -> Result<SlotRecord, FileVaultError> {
    let wrapped = wrap_dpapi(file_key)?;
    Ok(SlotRecord {
        kind: SlotKind::Dpapi,
        kdf: None,
        nonce_b64: None,
        wrapped_key_b64: b64(&wrapped),
    })
}

fn unlock_file_key(
    header: &VariableHeader,
    unlock: &UnlockOptions,
) -> Result<Vec<u8>, FileVaultError> {
    let mut attempts = Vec::new();
    if let Some(password) = unlock.password.as_deref() {
        attempts.push((
            SlotKind::Password,
            UnlockMaterial::Raw(password.as_bytes().to_vec()),
        ));
    }
    if let Some(keyfile) = unlock.keyfile.as_deref() {
        attempts.push((SlotKind::Keyfile, UnlockMaterial::Raw(fs::read(keyfile)?)));
    }
    if let Some(recovery_key) = unlock.recovery_key.as_deref() {
        attempts.push((
            SlotKind::RecoveryKey,
            UnlockMaterial::Recovery(recovery_key.trim().to_string()),
        ));
    }
    if unlock.dpapi {
        attempts.push((SlotKind::Dpapi, UnlockMaterial::Dpapi));
    }
    if attempts.is_empty() {
        return Err(FileVaultError::Slot(
            "no unlock material provided".to_string(),
        ));
    }

    for slot in &header.slots {
        for (kind, material) in &attempts {
            if slot.kind != *kind {
                continue;
            }
            if let Ok(file_key) = unwrap_slot(slot, material) {
                return Ok(file_key);
            }
        }
    }
    Err(FileVaultError::Slot(
        "no slot matched supplied unlock material".to_string(),
    ))
}

enum UnlockMaterial {
    Raw(Vec<u8>),
    Recovery(String),
    Dpapi,
}

fn unwrap_slot(slot: &SlotRecord, material: &UnlockMaterial) -> Result<Vec<u8>, FileVaultError> {
    match (slot.kind, material) {
        (SlotKind::Password, UnlockMaterial::Raw(bytes))
        | (SlotKind::Keyfile, UnlockMaterial::Raw(bytes)) => {
            let kdf = slot.kdf.as_ref().ok_or_else(|| {
                FileVaultError::Slot("missing kdf config for wrapped slot".to_string())
            })?;
            let wrap_key = derive_password_key(bytes, kdf)?;
            unwrap_key_material(&wrap_key, slot, slot.kind)
        }
        (SlotKind::RecoveryKey, UnlockMaterial::Recovery(value)) => {
            let wrap_key = derive_recovery_key(value)?;
            unwrap_key_material(&wrap_key, slot, slot.kind)
        }
        (SlotKind::Dpapi, UnlockMaterial::Dpapi) => {
            unwrap_dpapi(&vec_from_b64(&slot.wrapped_key_b64)?)
        }
        _ => Err(FileVaultError::Slot(
            "unlock material type mismatch".to_string(),
        )),
    }
}

fn derive_password_key(material: &[u8], config: &KdfConfig) -> Result<Vec<u8>, FileVaultError> {
    let salt = config.salt()?;
    let mut out = vec![0u8; 32];
    match config.kind {
        KdfKind::Argon2id => {
            let params = Argon2Params::new(
                config.mem_cost_kib,
                config.time_cost,
                config.parallelism.max(1),
                Some(32),
            )
            .map_err(|_| FileVaultError::Crypto("argon2 params"))?;
            let argon = Argon2::new(Argon2Algorithm::Argon2id, Version::V0x13, params);
            argon
                .hash_password_into(material, &salt, &mut out)
                .map_err(|_| FileVaultError::Crypto("argon2 derive"))?;
        }
        KdfKind::Pbkdf2Sha256 => {
            pbkdf2_hmac::<Sha256>(material, &salt, config.rounds, &mut out);
        }
    }
    Ok(out)
}

fn derive_recovery_key(value: &str) -> Result<Vec<u8>, FileVaultError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FileVaultError::Slot("empty recovery key".to_string()));
    }
    let decoded = b64_decode(trimmed)?;
    Ok(sha256_vec(&decoded))
}

fn encrypt_frame(
    algorithm: PayloadAlgorithm,
    key: &[u8],
    nonce_seed: &[u8; 16],
    sequence: u32,
    plaintext: &[u8],
) -> Result<Vec<u8>, FileVaultError> {
    let nonce = derive_frame_nonce(nonce_seed, sequence, algorithm);
    let aad = frame_aad(sequence, plaintext.len() as u32);
    match algorithm {
        PayloadAlgorithm::Aes256Gcm => {
            let cipher = Aes256Gcm::new_from_slice(key)
                .map_err(|_| FileVaultError::Crypto("aes256-gcm init"))?;
            cipher
                .encrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: plaintext,
                        aad: &aad,
                    },
                )
                .map_err(|_| FileVaultError::Crypto("aes256-gcm encrypt"))
        }
        PayloadAlgorithm::XChaCha20Poly1305 => {
            let cipher = XChaCha20Poly1305::new_from_slice(key)
                .map_err(|_| FileVaultError::Crypto("xchacha20 init"))?;
            cipher
                .encrypt(
                    XNonce::from_slice(&nonce),
                    chacha20poly1305::aead::Payload {
                        msg: plaintext,
                        aad: &aad,
                    },
                )
                .map_err(|_| FileVaultError::Crypto("xchacha20 encrypt"))
        }
    }
}

fn decrypt_frame(
    algorithm: PayloadAlgorithm,
    key: &[u8],
    nonce_seed: &[u8; 16],
    sequence: u32,
    ciphertext: &[u8],
    plaintext_len: u32,
) -> Result<Vec<u8>, FileVaultError> {
    let nonce = derive_frame_nonce(nonce_seed, sequence, algorithm);
    let aad = frame_aad(sequence, plaintext_len);
    match algorithm {
        PayloadAlgorithm::Aes256Gcm => {
            let cipher = Aes256Gcm::new_from_slice(key)
                .map_err(|_| FileVaultError::Crypto("aes256-gcm init"))?;
            cipher
                .decrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: ciphertext,
                        aad: &aad,
                    },
                )
                .map_err(|_| FileVaultError::Crypto("aes256-gcm decrypt"))
        }
        PayloadAlgorithm::XChaCha20Poly1305 => {
            let cipher = XChaCha20Poly1305::new_from_slice(key)
                .map_err(|_| FileVaultError::Crypto("xchacha20 init"))?;
            cipher
                .decrypt(
                    XNonce::from_slice(&nonce),
                    chacha20poly1305::aead::Payload {
                        msg: ciphertext,
                        aad: &aad,
                    },
                )
                .map_err(|_| FileVaultError::Crypto("xchacha20 decrypt"))
        }
    }
}

fn wrap_key_material(
    wrap_key: &[u8],
    file_key: &[u8],
    kind: SlotKind,
) -> Result<(Vec<u8>, Vec<u8>), FileVaultError> {
    let nonce = random_bytes(12)?;
    let cipher = Aes256Gcm::new_from_slice(wrap_key)
        .map_err(|_| FileVaultError::Crypto("slot aes-gcm init"))?;
    let wrapped = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: file_key,
                aad: kind.as_str().as_bytes(),
            },
        )
        .map_err(|_| FileVaultError::Crypto("slot wrap encrypt"))?;
    Ok((nonce, wrapped))
}

fn unwrap_key_material(
    wrap_key: &[u8],
    slot: &SlotRecord,
    kind: SlotKind,
) -> Result<Vec<u8>, FileVaultError> {
    let nonce = vec_from_b64(
        slot.nonce_b64
            .as_deref()
            .ok_or_else(|| FileVaultError::Slot("wrapped slot missing nonce".to_string()))?,
    )?;
    let wrapped = vec_from_b64(&slot.wrapped_key_b64)?;
    let cipher = Aes256Gcm::new_from_slice(wrap_key)
        .map_err(|_| FileVaultError::Crypto("slot aes-gcm init"))?;
    cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &wrapped,
                aad: kind.as_str().as_bytes(),
            },
        )
        .map_err(|_| FileVaultError::Slot("slot unwrap failed".to_string()))
}

fn encrypt_filename_metadata(
    path: &Path,
    file_key: &[u8],
) -> Result<Option<FilenameInfo>, FileVaultError> {
    let name = match path.file_name().and_then(|value| value.to_str()) {
        Some(name) if !name.is_empty() => name,
        _ => return Ok(None),
    };
    let aad = path
        .parent()
        .map(|parent| parent.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    let filename_key = derive_hmac(file_key, FILENAME_KEY_INFO)?;
    let nonce = random_bytes(16)?;
    let cipher = Aes128SivAead::new_from_slice(&filename_key[..32])
        .map_err(|_| FileVaultError::Crypto("aes-siv init"))?;
    let ciphertext = cipher
        .encrypt(
            SivNonce::from_slice(&nonce),
            SivPayload {
                msg: name.as_bytes(),
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| FileVaultError::Crypto("aes-siv encrypt"))?;
    Ok(Some(FilenameInfo {
        nonce_b64: b64(&nonce),
        ciphertext_b64: b64(&ciphertext),
        aad_b64: b64(aad.as_bytes()),
    }))
}

fn compute_payload_digest(
    path: &Path,
    first_frame_offset: u64,
    frame_count: u32,
    footer_offset: Option<u64>,
) -> Result<[u8; 32], FileVaultError> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(first_frame_offset))?;
    let mut hasher = Sha256::new();
    let mut ciphertext = Vec::new();
    for expected_seq in 0..frame_count {
        let mut header_bytes = [0u8; FRAME_HEADER_LEN];
        file.read_exact(&mut header_bytes)
            .map_err(|_| FileVaultError::Verify("frame header truncated".to_string()))?;
        let header = decode_frame_header(header_bytes)?;
        if header.sequence != expected_seq {
            return Err(FileVaultError::Verify(format!(
                "frame sequence mismatch at index {expected_seq}"
            )));
        }
        let needed = header.ciphertext_len as usize;
        if ciphertext.len() != needed {
            ciphertext.resize(needed, 0);
        }
        file.read_exact(&mut ciphertext)
            .map_err(|_| FileVaultError::Verify("frame payload truncated".to_string()))?;
        if crc32(&ciphertext) != header.ciphertext_crc32 {
            return Err(FileVaultError::Verify(format!(
                "frame crc mismatch at index {expected_seq}"
            )));
        }
        hasher.update(header_bytes);
        hasher.update(&ciphertext);
    }
    if let Some(footer_offset) = footer_offset {
        let current = file.stream_position()?;
        if current != footer_offset {
            return Err(FileVaultError::Verify(
                "frame layout length mismatch".to_string(),
            ));
        }
    }
    Ok(hasher.finalize().into())
}

fn compute_header_mac(
    payload_digest: &[u8; 32],
    fixed_header_bytes: &[u8; FIXED_HEADER_LEN],
    variable_header_bytes: &[u8],
) -> Result<[u8; HEADER_MAC_LEN], FileVaultError> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(payload_digest)
        .map_err(|_| FileVaultError::Crypto("header mac key init"))?;
    mac.update(HEADER_MAC_INFO);
    mac.update(fixed_header_bytes);
    mac.update(variable_header_bytes);
    Ok(mac.finalize().into_bytes().into())
}

fn encode_fixed_header(header: &FixedHeader) -> [u8; FIXED_HEADER_LEN] {
    let mut out = [0u8; FIXED_HEADER_LEN];
    out[..8].copy_from_slice(MAGIC);
    out[8..10].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
    out[10..12].copy_from_slice(&header.flags.to_le_bytes());
    out[12] = header.mode.as_u8();
    out[13] = header.payload_algorithm.as_u8();
    out[14] = header.header_mac_algorithm.as_u8();
    out[15] = 0;
    out[16..20].copy_from_slice(&header.chunk_size.to_le_bytes());
    out[20..24].copy_from_slice(&header.variable_header_len.to_le_bytes());
    out[24..32].copy_from_slice(&header.plaintext_len.to_le_bytes());
    out[32..36].copy_from_slice(&header.frame_count.to_le_bytes());
    out[36..40].copy_from_slice(&(HEADER_MAC_LEN as u32).to_le_bytes());
    out[40..48].copy_from_slice(&header.created_unix.to_le_bytes());
    out[48..64].copy_from_slice(&header.nonce_seed);
    out
}

fn decode_fixed_header(bytes: [u8; FIXED_HEADER_LEN]) -> Result<FixedHeader, FileVaultError> {
    if &bytes[..8] != MAGIC {
        return Err(FileVaultError::InvalidFormat("magic mismatch".to_string()));
    }
    let version = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
    if version != FORMAT_VERSION {
        return Err(FileVaultError::InvalidFormat(format!(
            "unsupported version: {version}"
        )));
    }
    let header_mac_len = u32::from_le_bytes(bytes[36..40].try_into().unwrap());
    if header_mac_len as usize != HEADER_MAC_LEN {
        return Err(FileVaultError::InvalidFormat(
            "header mac length mismatch".to_string(),
        ));
    }
    let mut nonce_seed = [0u8; 16];
    nonce_seed.copy_from_slice(&bytes[48..64]);
    Ok(FixedHeader {
        flags: u16::from_le_bytes(bytes[10..12].try_into().unwrap()),
        mode: VaultMode::from_u8(bytes[12])?,
        payload_algorithm: PayloadAlgorithm::from_u8(bytes[13])?,
        header_mac_algorithm: HeaderMacAlgorithm::from_u8(bytes[14])?,
        chunk_size: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
        variable_header_len: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
        plaintext_len: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
        frame_count: u32::from_le_bytes(bytes[32..36].try_into().unwrap()),
        created_unix: u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
        nonce_seed,
    })
}

fn encode_frame_header(header: &FrameHeader) -> [u8; FRAME_HEADER_LEN] {
    let mut out = [0u8; FRAME_HEADER_LEN];
    out[..4].copy_from_slice(&header.sequence.to_le_bytes());
    out[4..8].copy_from_slice(&header.plaintext_len.to_le_bytes());
    out[8..12].copy_from_slice(&header.ciphertext_len.to_le_bytes());
    out[12..16].copy_from_slice(&header.ciphertext_crc32.to_le_bytes());
    out
}

fn decode_frame_header(bytes: [u8; FRAME_HEADER_LEN]) -> Result<FrameHeader, FileVaultError> {
    Ok(FrameHeader {
        sequence: u32::from_le_bytes(bytes[..4].try_into().unwrap()),
        plaintext_len: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        ciphertext_len: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        ciphertext_crc32: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
    })
}

fn read_frame_header(file: &mut File) -> Result<FrameHeader, FileVaultError> {
    let mut header_bytes = [0u8; FRAME_HEADER_LEN];
    file.read_exact(&mut header_bytes)?;
    decode_frame_header(header_bytes)
}

fn encode_footer(footer: &CommitFooter) -> [u8; FOOTER_LEN] {
    let mut out = [0u8; FOOTER_LEN];
    out[..8].copy_from_slice(COMMIT_MAGIC);
    out[8..40].copy_from_slice(&footer.payload_digest);
    out[40..72].copy_from_slice(&footer.header_mac);
    out
}

fn decode_footer(bytes: [u8; FOOTER_LEN]) -> Result<CommitFooter, FileVaultError> {
    if &bytes[..8] != COMMIT_MAGIC {
        return Err(FileVaultError::InvalidFormat(
            "footer magic mismatch".to_string(),
        ));
    }
    let mut payload_digest = [0u8; 32];
    payload_digest.copy_from_slice(&bytes[8..40]);
    let mut header_mac = [0u8; 32];
    header_mac.copy_from_slice(&bytes[40..72]);
    Ok(CommitFooter {
        payload_digest,
        header_mac,
    })
}

fn derive_frame_nonce(seed: &[u8; 16], sequence: u32, algorithm: PayloadAlgorithm) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(FRAME_NONCE_INFO);
    hasher.update(seed);
    hasher.update(sequence.to_le_bytes());
    let digest = hasher.finalize();
    match algorithm {
        PayloadAlgorithm::Aes256Gcm => digest[..12].to_vec(),
        PayloadAlgorithm::XChaCha20Poly1305 => digest[..24].to_vec(),
    }
}

fn frame_aad(sequence: u32, plaintext_len: u32) -> [u8; 8] {
    let mut aad = [0u8; 8];
    aad[..4].copy_from_slice(&sequence.to_le_bytes());
    aad[4..8].copy_from_slice(&plaintext_len.to_le_bytes());
    aad
}

fn compute_frame_count(plaintext_len: u64, chunk_size: u32) -> u32 {
    if plaintext_len == 0 {
        0
    } else {
        plaintext_len.div_ceil(chunk_size as u64) as u32
    }
}

fn temp_path_for(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.fvtmp", path.to_string_lossy()))
}

fn journal_path_for(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.fvjournal", path.to_string_lossy()))
}

fn cleanup_sidecars(path: &Path) -> Result<(), FileVaultError> {
    let temp = temp_path_for(path);
    let journal = journal_path_for(path);
    if temp.exists() {
        fs::remove_file(temp)?;
    }
    if journal.exists() {
        fs::remove_file(journal)?;
    }
    Ok(())
}

fn write_journal(path: &Path, journal: &Journal) -> Result<(), FileVaultError> {
    let bytes = serde_json::to_vec_pretty(journal)?;
    fs::write(path, bytes)?;
    Ok(())
}

fn read_journal(path: &Path) -> Result<Journal, FileVaultError> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn commit_temp_into_place(temp_path: &Path, output_path: &Path) -> Result<(), FileVaultError> {
    if output_path.exists() {
        fs::remove_file(output_path)?;
    }
    fs::rename(temp_path, output_path)?;
    Ok(())
}

fn replace_existing_file(temp_path: &Path, output_path: &Path) -> Result<(), FileVaultError> {
    if output_path.exists() {
        fs::remove_file(output_path)?;
    }
    fs::rename(temp_path, output_path)?;
    Ok(())
}

fn copy_exact_bytes(
    source: &mut File,
    target: &mut File,
    mut remaining: u64,
) -> Result<(), FileVaultError> {
    let mut buffer = vec![0u8; 256 * 1024];
    while remaining > 0 {
        let to_read = remaining.min(buffer.len() as u64) as usize;
        source.read_exact(&mut buffer[..to_read])?;
        target.write_all(&buffer[..to_read])?;
        remaining -= to_read as u64;
    }
    Ok(())
}

fn unlock_supplied(unlock: &UnlockOptions) -> bool {
    unlock.password.is_some()
        || unlock.keyfile.is_some()
        || unlock.recovery_key.is_some()
        || unlock.dpapi
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

fn random_bytes(len: usize) -> Result<Vec<u8>, FileVaultError> {
    let mut out = vec![0u8; len];
    fill_random(&mut out).map_err(|_| FileVaultError::Crypto("rng fill"))?;
    Ok(out)
}

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn b64_decode(value: &str) -> Result<Vec<u8>, FileVaultError> {
    URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| FileVaultError::InvalidFormat("base64 decode failed".to_string()))
}

fn vec_from_b64(value: &str) -> Result<Vec<u8>, FileVaultError> {
    b64_decode(value)
}

fn derive_hmac(key: &[u8], context: &[u8]) -> Result<Vec<u8>, FileVaultError> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key)
        .map_err(|_| FileVaultError::Crypto("hmac init"))?;
    mac.update(context);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn sha256_vec(data: &[u8]) -> Vec<u8> {
    Sha256::digest(data).to_vec()
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut hasher = Crc32Hasher::new();
    hasher.update(bytes);
    hasher.finalize()
}

fn hex_string(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn generate_recovery_key() -> Result<String, FileVaultError> {
    Ok(b64(&random_bytes(32)?))
}

#[cfg(windows)]
fn wrap_dpapi(bytes: &[u8]) -> Result<Vec<u8>, FileVaultError> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(
            &input,
            PCWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|_| FileVaultError::Crypto("CryptProtectData failed"))?;
        let wrapped = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(output.pbData as *mut _));
        Ok(wrapped)
    }
}

#[cfg(not(windows))]
fn wrap_dpapi(_bytes: &[u8]) -> Result<Vec<u8>, FileVaultError> {
    Err(FileVaultError::Unsupported(
        "dpapi is only available on Windows".to_string(),
    ))
}

#[cfg(windows)]
fn unwrap_dpapi(bytes: &[u8]) -> Result<Vec<u8>, FileVaultError> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|_| FileVaultError::Slot("dpapi unwrap failed".to_string()))?;
        let plaintext = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(output.pbData as *mut _));
        Ok(plaintext)
    }
}

#[cfg(not(windows))]
fn unwrap_dpapi(_bytes: &[u8]) -> Result<Vec<u8>, FileVaultError> {
    Err(FileVaultError::Unsupported(
        "dpapi is only available on Windows".to_string(),
    ))
}
