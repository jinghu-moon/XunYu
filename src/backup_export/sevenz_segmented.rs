use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub(crate) struct SegmentedWriter {
    base_path: PathBuf,
    split_size: u64,
    current_index: usize,
    current_file: File,
    global_pos: u64,
    total_len: u64,
    volume_lengths: Vec<u64>,
    volume_paths: Vec<PathBuf>,
}

#[allow(dead_code)]
impl SegmentedWriter {
    pub(crate) fn create(base_path: impl AsRef<Path>, split_size: u64) -> io::Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        let first_path = volume_path(&base_path, 0);
        if let Some(parent) = first_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let current_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(&first_path)?;
        Ok(Self {
            base_path,
            split_size,
            current_index: 0,
            current_file,
            global_pos: 0,
            total_len: 0,
            volume_lengths: vec![0],
            volume_paths: vec![first_path],
        })
    }

    pub(crate) fn finish(mut self) -> io::Result<Vec<PathBuf>> {
        self.current_file.flush()?;
        Ok(self.volume_paths)
    }

    #[allow(dead_code)]
    pub(crate) fn logical_position(&self) -> u64 {
        self.global_pos
    }

    fn seek_to_volume(&mut self, index: usize, offset: u64) -> io::Result<()> {
        if index >= self.volume_paths.len() {
            self.ensure_volume(index)?;
        }
        if self.current_index != index {
            self.current_file.flush()?;
            self.current_file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .open(&self.volume_paths[index])?;
            self.current_index = index;
        }
        self.current_file.seek(SeekFrom::Start(offset))?;
        Ok(())
    }

    fn ensure_volume(&mut self, index: usize) -> io::Result<()> {
        while self.volume_paths.len() <= index {
            let next_index = self.volume_paths.len();
            let path = volume_path(&self.base_path, next_index);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(true)
                .open(&path)?;
            self.volume_paths.push(path);
            self.volume_lengths.push(0);
        }
        Ok(())
    }

    fn write_gap_zeros(&mut self, target_pos: u64) -> io::Result<()> {
        if target_pos <= self.total_len {
            return Ok(());
        }
        let saved_pos = self.global_pos;
        self.global_pos = self.total_len;
        let gap = target_pos - self.total_len;
        let zeros = [0u8; 8192];
        let mut remaining = gap;
        while remaining > 0 {
            let chunk = remaining.min(zeros.len() as u64) as usize;
            self.write_all(&zeros[..chunk])?;
            remaining -= chunk as u64;
        }
        self.global_pos = saved_pos;
        self.seek(self.global_pos_to_seek(saved_pos))?;
        Ok(())
    }

    fn global_pos_to_seek(&self, pos: u64) -> SeekFrom {
        SeekFrom::Start(pos)
    }
}

impl Write for SegmentedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.global_pos > self.total_len {
            self.write_gap_zeros(self.global_pos)?;
        }

        let mut written = 0usize;
        while written < buf.len() {
            let volume_index = (self.global_pos / self.split_size) as usize;
            let volume_offset = self.global_pos % self.split_size;
            let volume_remaining = (self.split_size - volume_offset) as usize;
            let chunk = volume_remaining.min(buf.len() - written);

            self.seek_to_volume(volume_index, volume_offset)?;
            self.current_file.write_all(&buf[written..written + chunk])?;

            let new_len = volume_offset + chunk as u64;
            if self.volume_lengths[volume_index] < new_len {
                self.volume_lengths[volume_index] = new_len;
            }
            self.global_pos += chunk as u64;
            if self.total_len < self.global_pos {
                self.total_len = self.global_pos;
            }
            written += chunk;
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.current_file.flush()
    }
}

impl Seek for SegmentedWriter {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let next = match pos {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::Current(value) => self.global_pos as i128 + value as i128,
            SeekFrom::End(value) => self.total_len as i128 + value as i128,
        };
        if next < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before start is not supported",
            ));
        }
        self.global_pos = next as u64;
        Ok(self.global_pos)
    }
}

#[allow(dead_code)]
pub(crate) struct MultiVolumeReader {
    volume_paths: Vec<PathBuf>,
    volume_lengths: Vec<u64>,
    volume_offsets: Vec<u64>,
    current_index: usize,
    current_file: File,
    position: u64,
    total_len: u64,
}

#[allow(dead_code)]
impl MultiVolumeReader {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let base = resolve_multivolume_base(path).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "7z split volume set not found")
        })?;
        let volume_paths = list_numbered_outputs(&base)?;
        if volume_paths.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "7z split volume set is empty",
            ));
        }
        let mut volume_lengths = Vec::with_capacity(volume_paths.len());
        let mut volume_offsets = Vec::with_capacity(volume_paths.len());
        let mut offset = 0u64;
        for path in &volume_paths {
            volume_offsets.push(offset);
            let len = fs::metadata(path)?.len();
            volume_lengths.push(len);
            offset += len;
        }
        let current_file = File::open(&volume_paths[0])?;
        Ok(Self {
            volume_paths,
            volume_lengths,
            volume_offsets,
            current_index: 0,
            current_file,
            position: 0,
            total_len: offset,
        })
    }

    fn seek_to_position(&mut self, position: u64) -> io::Result<()> {
        let index = locate_volume_index(&self.volume_offsets, &self.volume_lengths, position);
        if self.current_index != index {
            self.current_file = File::open(&self.volume_paths[index])?;
            self.current_index = index;
        }
        let volume_offset = position - self.volume_offsets[index];
        self.current_file.seek(SeekFrom::Start(volume_offset))?;
        Ok(())
    }
}

impl Read for MultiVolumeReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.position >= self.total_len {
            return Ok(0);
        }

        let mut total_read = 0usize;
        while total_read < buf.len() && self.position < self.total_len {
            self.seek_to_position(self.position)?;
            let index = self.current_index;
            let volume_offset = self.position - self.volume_offsets[index];
            let remaining_in_volume = (self.volume_lengths[index] - volume_offset) as usize;
            let want = remaining_in_volume.min(buf.len() - total_read);
            let read = self.current_file.read(&mut buf[total_read..total_read + want])?;
            if read == 0 {
                break;
            }
            self.position += read as u64;
            total_read += read;
        }
        Ok(total_read)
    }
}

impl Seek for MultiVolumeReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let next = match pos {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::Current(value) => self.position as i128 + value as i128,
            SeekFrom::End(value) => self.total_len as i128 + value as i128,
        };
        if next < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before start is not supported",
            ));
        }
        let next = next as u64;
        if next > self.total_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek beyond end is not supported",
            ));
        }
        self.position = next;
        Ok(self.position)
    }
}

pub(crate) fn volume_path(base_path: &Path, index: usize) -> PathBuf {
    PathBuf::from(format!("{}.{:03}", base_path.display(), index + 1))
}

pub(crate) fn list_numbered_outputs(base_path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut outputs = Vec::new();
    let Some(parent) = base_path.parent() else {
        return Ok(outputs);
    };
    let Some(prefix) = base_path.file_name().and_then(|name| name.to_str()) else {
        return Ok(outputs);
    };
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{prefix}."))
            && name.len() == prefix.len() + 4
            && name[prefix.len() + 1..]
                .chars()
                .all(|ch| ch.is_ascii_digit())
        {
            outputs.push(entry.path());
        }
    }
    outputs.sort();
    Ok(outputs)
}

pub(crate) fn resolve_multivolume_base(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_str()?;
    if name.ends_with(".7z.001") {
        return Some(PathBuf::from(path.to_string_lossy().trim_end_matches(".001")));
    }
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("7z"))
        && volume_path(path, 0).exists()
    {
        return Some(path.to_path_buf());
    }
    None
}

#[allow(dead_code)]
fn locate_volume_index(offsets: &[u64], lengths: &[u64], position: u64) -> usize {
    if position == 0 {
        return 0;
    }
    for index in 0..offsets.len() {
        let start = offsets[index];
        let end = start + lengths[index];
        if position >= start && position < end {
            return index;
        }
    }
    offsets.len().saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Seek, SeekFrom, Write};

    use tempfile::tempdir;

    use super::{MultiVolumeReader, SegmentedWriter, list_numbered_outputs, resolve_multivolume_base};

    #[test]
    fn segmented_writer_splits_logical_stream_into_numbered_volumes() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("archive.7z");
        let mut writer = SegmentedWriter::create(&base, 5).unwrap();
        writer.write_all(b"hello world").unwrap();
        let outputs = writer.finish().unwrap();

        assert_eq!(outputs.len(), 3);
        assert!(dir.path().join("archive.7z.001").exists());
        assert!(dir.path().join("archive.7z.002").exists());
        assert!(dir.path().join("archive.7z.003").exists());
    }

    #[test]
    fn segmented_writer_supports_header_rewrite_seek_pattern() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("archive.7z");
        let mut writer = SegmentedWriter::create(&base, 8).unwrap();
        writer.seek(SeekFrom::Start(4)).unwrap();
        writer.write_all(b"abcdefgh").unwrap();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.write_all(b"HEAD").unwrap();
        let outputs = writer.finish().unwrap();

        let first = fs::read(&outputs[0]).unwrap();
        assert_eq!(&first[..8], b"HEADabcd");
    }

    #[test]
    fn segmented_writer_reports_logical_position() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("archive.7z");
        let mut writer = SegmentedWriter::create(&base, 8).unwrap();
        writer.seek(SeekFrom::Start(4)).unwrap();
        assert_eq!(writer.logical_position(), 4);
        writer.write_all(b"abc").unwrap();
        assert_eq!(writer.logical_position(), 7);
    }

    #[test]
    fn segmented_writer_finish_returns_volume_paths() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("archive.7z");
        let mut writer = SegmentedWriter::create(&base, 5).unwrap();
        writer.write_all(b"hello world").unwrap();
        let outputs = writer.finish().unwrap();
        assert_eq!(outputs.len(), 3);
        assert!(outputs[0].ends_with("archive.7z.001"));
        assert!(outputs[1].ends_with("archive.7z.002"));
        assert!(outputs[2].ends_with("archive.7z.003"));
    }

    #[test]
    fn multivolume_reader_reads_across_volume_boundaries() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("archive.7z");
        fs::write(dir.path().join("archive.7z.001"), b"hello").unwrap();
        fs::write(dir.path().join("archive.7z.002"), b" world").unwrap();

        let mut reader = MultiVolumeReader::open(&base).unwrap();
        let mut content = String::new();
        reader.read_to_string(&mut content).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn resolve_multivolume_base_accepts_base_and_first_volume() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("archive.7z");
        fs::write(dir.path().join("archive.7z.001"), b"part1").unwrap();

        assert_eq!(resolve_multivolume_base(&base).unwrap(), base);
        assert_eq!(
            resolve_multivolume_base(&dir.path().join("archive.7z.001")).unwrap(),
            dir.path().join("archive.7z")
        );
    }

    #[test]
    fn list_numbered_outputs_returns_sorted_paths() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("archive.7z");
        fs::write(dir.path().join("archive.7z.002"), b"part2").unwrap();
        fs::write(dir.path().join("archive.7z.001"), b"part1").unwrap();

        let paths = list_numbered_outputs(&base).unwrap();
        let names: Vec<String> = paths
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["archive.7z.001", "archive.7z.002"]);
    }
}
