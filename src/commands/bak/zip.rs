use std::fs;
use std::io;
use std::path::Path;

pub(crate) fn compress_dir(dir: &Path, zip_path: &Path) -> io::Result<()> {
    let file = fs::File::create(zip_path)?;
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    fn walk_zip(
        dir: &Path,
        base: &Path,
        archive: &mut zip::ZipWriter<fs::File>,
        options: zip::write::SimpleFileOptions,
    ) -> io::Result<()> {
        for e in fs::read_dir(dir)?.flatten() {
            let path = e.path();
            let rel = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                walk_zip(&path, base, archive, options)?;
            } else {
                archive.start_file(&rel, options)?;
                let mut f = fs::File::open(&path)?;
                io::copy(&mut f, archive)?;
            }
        }
        Ok(())
    }

    walk_zip(dir, dir, &mut archive, options)?;
    archive.finish()?;
    Ok(())
}
