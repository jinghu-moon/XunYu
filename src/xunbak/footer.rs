use crate::xunbak::constants::{FOOTER_MAGIC, FOOTER_SIZE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Footer {
    pub checkpoint_offset: u64,
}

#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq, Eq)]
pub enum FooterError {
    #[error("footer too short: {actual}")]
    FooterTooShort { actual: usize },
    #[error("invalid footer magic")]
    InvalidFooterMagic,
    #[error("footer CRC mismatch")]
    FooterCrcMismatch,
    #[error("checkpoint offset out of range: offset={checkpoint_offset}, file_size={file_size}")]
    OffsetOutOfRange {
        checkpoint_offset: u64,
        file_size: u64,
    },
}

impl Footer {
    pub fn to_bytes(self) -> [u8; FOOTER_SIZE] {
        let mut bytes = [0u8; FOOTER_SIZE];
        bytes[..8].copy_from_slice(&FOOTER_MAGIC);
        bytes[8..16].copy_from_slice(&self.checkpoint_offset.to_le_bytes());
        let crc = crc32c::crc32c(&bytes[..16]);
        bytes[16..20].copy_from_slice(&crc.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8], file_size: u64) -> Result<Self, FooterError> {
        if bytes.len() < FOOTER_SIZE {
            return Err(FooterError::FooterTooShort {
                actual: bytes.len(),
            });
        }
        if bytes[..8] != FOOTER_MAGIC {
            return Err(FooterError::InvalidFooterMagic);
        }

        let expected_crc = u32::from_le_bytes(bytes[16..20].try_into().expect("len checked"));
        let actual_crc = crc32c::crc32c(&bytes[..16]);
        if expected_crc != actual_crc {
            return Err(FooterError::FooterCrcMismatch);
        }

        let checkpoint_offset = u64::from_le_bytes(bytes[8..16].try_into().expect("len checked"));
        if checkpoint_offset >= file_size {
            return Err(FooterError::OffsetOutOfRange {
                checkpoint_offset,
                file_size,
            });
        }

        Ok(Self { checkpoint_offset })
    }
}
