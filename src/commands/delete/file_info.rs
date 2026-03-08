use std::fmt;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FileKind {
    PeExe,
    PeDriver,
    Elf,
    Zip,
    Pdf,
    SevenZip,
    Rar,
    GzipTar,
    MsOffice,
    Empty,
    Text,
    Unknown,
}

impl fmt::Display for FileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FileKind::PeExe => "PE executable",
            FileKind::PeDriver => "PE driver",
            FileKind::Elf => "ELF executable",
            FileKind::Zip => "ZIP archive",
            FileKind::Pdf => "PDF",
            FileKind::SevenZip => "7z archive",
            FileKind::Rar => "RAR archive",
            FileKind::GzipTar => "Gzip/Tar",
            FileKind::MsOffice => "MS Office (OLE2)",
            FileKind::Empty => "Empty",
            FileKind::Text => "Text",
            FileKind::Unknown => "Unknown",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FileInfo {
    pub(crate) sha256: String,
    pub(crate) kind: FileKind,
    pub(crate) size: u64,
}

pub(crate) fn collect(path: &Path) -> Option<FileInfo> {
    let mut file = std::fs::File::open(path).ok()?;
    let size = file.metadata().ok()?.len();

    if size == 0 {
        return Some(FileInfo {
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
            kind: FileKind::Empty,
            size: 0,
        });
    }

    let mut head = [0u8; 8192];
    let head_len = file.read(&mut head).ok()?;
    let head = &head[..head_len];

    let kind = identify(head, path);
    let sha256 = sha256_file(path)?;

    Some(FileInfo { sha256, kind, size })
}

fn identify(head: &[u8], path: &Path) -> FileKind {
    if head.len() < 2 {
        return FileKind::Unknown;
    }

    if head.starts_with(b"MZ") {
        if head.len() >= 0x40 {
            let pe_off = u32::from_le_bytes(head[0x3C..0x40].try_into().unwrap_or([0; 4])) as usize;
            if pe_off + 6 < head.len() && &head[pe_off..pe_off + 4] == b"PE\0\0" {
                let sub_off = pe_off + 0x5C;
                if sub_off + 2 <= head.len() {
                    let sub =
                        u16::from_le_bytes(head[sub_off..sub_off + 2].try_into().unwrap_or([0; 2]));
                    if sub == 1 {
                        return FileKind::PeDriver;
                    }
                }
            }
        }
        return FileKind::PeExe;
    }

    if head.starts_with(b"\x7FELF") {
        return FileKind::Elf;
    }
    if head.starts_with(b"PK\x03\x04") {
        return FileKind::Zip;
    }
    if head.starts_with(b"%PDF") {
        return FileKind::Pdf;
    }
    if head.starts_with(b"7z\xBC\xAF\x27\x1C") {
        return FileKind::SevenZip;
    }
    if head.starts_with(b"Rar!\x1A\x07") {
        return FileKind::Rar;
    }
    if head.starts_with(b"\x1F\x8B") {
        return FileKind::GzipTar;
    }
    if head.starts_with(b"\xD0\xCF\x11\xE0") {
        return FileKind::MsOffice;
    }

    let sample = &head[..head.len().min(512)];
    if sample.iter().all(|&b| b >= 0x09 && b < 0x80) {
        return FileKind::Text;
    }

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "exe" | "dll" | "sys" => return FileKind::PeExe,
            "zip" | "jar" => return FileKind::Zip,
            "pdf" => return FileKind::Pdf,
            _ => {}
        }
    }

    FileKind::Unknown
}

fn sha256_file(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 4096];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(_) => break,
        }
    }
    Some(format!("{:x}", hasher.finalize()))
}
