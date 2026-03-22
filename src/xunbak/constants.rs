pub const HEADER_MAGIC: [u8; 8] = *b"XUNBAK\0\0";
pub const FOOTER_MAGIC: [u8; 8] = *b"XBKFTR\0\0";

pub const HEADER_SIZE: usize = 64;
pub const FOOTER_SIZE: usize = 24;
pub const RECORD_PREFIX_SIZE: usize = 13;
pub const BLOB_HEADER_SIZE: usize = 50;
pub const CHECKPOINT_PAYLOAD_SIZE: usize = 128;
pub const XUNBAK_WRITE_VERSION: u32 = 1;
pub const XUNBAK_READER_VERSION: u32 = 1;

pub const FLAG_SPLIT: u64 = 0x01;
pub const FLAG_ALIGNED: u64 = 0x02;
pub const KNOWN_HEADER_FLAGS: u64 = FLAG_SPLIT | FLAG_ALIGNED;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RecordType(u8);

impl RecordType {
    pub const BLOB: Self = Self(0x01);
    pub const MANIFEST: Self = Self(0x02);
    pub const CHECKPOINT: Self = Self(0x03);
    pub const RESERVED: Self = Self(0x04);
    pub const PACK: Self = Self(0x05);
    pub const INDEX: Self = Self(0x06);
    pub const TOMBSTONE: Self = Self(0x07);

    pub const fn from_u8(value: u8) -> Self {
        Self(value)
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }

    pub const fn is_known(self) -> bool {
        matches!(self.0, 0x01..=0x07)
    }
}

impl From<u8> for RecordType {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

impl From<RecordType> for u8 {
    fn from(value: RecordType) -> Self {
        value.as_u8()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Codec(u8);

impl Codec {
    pub const NONE: Self = Self(0x00);
    pub const ZSTD: Self = Self(0x01);
    pub const LZ4: Self = Self(0x02);
    pub const LZMA: Self = Self(0x03);

    pub const fn from_u8(value: u8) -> Self {
        Self(value)
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }

    pub const fn is_known(self) -> bool {
        matches!(self.0, 0x00..=0x03)
    }
}

impl From<u8> for Codec {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

impl From<Codec> for u8 {
    fn from(value: Codec) -> Self {
        value.as_u8()
    }
}
