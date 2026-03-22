use crate::xunbak::constants::{
    FLAG_SPLIT, HEADER_MAGIC, HEADER_SIZE, KNOWN_HEADER_FLAGS, XUNBAK_READER_VERSION,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitHeader {
    pub volume_index: u16,
    pub split_size: u64,
    pub set_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub write_version: u32,
    pub min_reader_version: u32,
    pub flags: u64,
    pub created_at_unix: u64,
    pub split: Option<SplitHeader>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedHeader {
    pub header: Header,
    pub unknown_flags: u64,
}

#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq, Eq)]
pub enum HeaderError {
    #[error("header too short: {actual}")]
    HeaderTooShort { actual: usize },
    #[error("invalid header magic")]
    InvalidMagic,
    #[error("reader version too old: need {min_reader_version}, current {current}")]
    VersionTooNew {
        min_reader_version: u32,
        current: u32,
    },
}

impl Header {
    pub fn to_bytes(self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[..8].copy_from_slice(&HEADER_MAGIC);
        bytes[8..12].copy_from_slice(&self.write_version.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.min_reader_version.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.flags.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.created_at_unix.to_le_bytes());

        if let Some(split) = self.split.filter(|_| self.flags & FLAG_SPLIT != 0) {
            bytes[32..34].copy_from_slice(&split.volume_index.to_le_bytes());
            bytes[40..48].copy_from_slice(&split.split_size.to_le_bytes());
            bytes[48..56].copy_from_slice(&split.set_id.to_le_bytes());
        }

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<DecodedHeader, HeaderError> {
        if bytes.len() < HEADER_SIZE {
            return Err(HeaderError::HeaderTooShort {
                actual: bytes.len(),
            });
        }

        if bytes[..8] != HEADER_MAGIC {
            return Err(HeaderError::InvalidMagic);
        }

        let write_version = u32::from_le_bytes(bytes[8..12].try_into().expect("len checked"));
        let min_reader_version = u32::from_le_bytes(bytes[12..16].try_into().expect("len checked"));
        if min_reader_version > XUNBAK_READER_VERSION {
            return Err(HeaderError::VersionTooNew {
                min_reader_version,
                current: XUNBAK_READER_VERSION,
            });
        }

        let flags = u64::from_le_bytes(bytes[16..24].try_into().expect("len checked"));
        let created_at_unix = u64::from_le_bytes(bytes[24..32].try_into().expect("len checked"));
        let split = if flags & FLAG_SPLIT != 0 {
            Some(SplitHeader {
                volume_index: u16::from_le_bytes(bytes[32..34].try_into().expect("len checked")),
                split_size: u64::from_le_bytes(bytes[40..48].try_into().expect("len checked")),
                set_id: u64::from_le_bytes(bytes[48..56].try_into().expect("len checked")),
            })
        } else {
            None
        };

        Ok(DecodedHeader {
            header: Header {
                write_version,
                min_reader_version,
                flags,
                created_at_unix,
                split,
            },
            unknown_flags: flags & !KNOWN_HEADER_FLAGS,
        })
    }
}
