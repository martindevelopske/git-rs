use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest, Sha1};
#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;

struct HashWriter<W> {
    writer: W,
    hasher: Sha1,
}

impl<W> Write for HashWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

pub fn invoke(write: bool, file: PathBuf) -> anyhow::Result<()> {
    // Reads the file.
    //
    // Constructs a blob header: blob <size>\0
    //
    // Appends the file content.
    //
    // Computes the SHA-1 hash of the entire blob.
    //
    // Compresses the blob using zlib.
    //
    // Stores it in .git/objects/ under a path based on the hash.
    //

    //blob structure: blob <size>\0<content>
    //should return generated hash of the above.
    fn write_blob<W>(file: &Path, writer: W) -> anyhow::Result<String>
    where
        W: Write,
    {
        let stat = std::fs::metadata(file).with_context(|| format!("stat {}", file.display()))?;
        let writer = ZlibEncoder::new(writer, Compression::default());
        let mut writer = HashWriter {
            writer,
            hasher: Sha1::new(),
        };

        write!(writer, "blob ")?;
        write!(writer, "{}\0", stat.len())?;
        let mut content = std::fs::File::open(&file).with_context(|| "Opening file")?;
        std::io::copy(&mut content, &mut writer).context("copying content")?;
        let _ = writer.writer.finish();
        let hash = writer.hasher.finalize();
        Ok(hex::encode(hash))
    }

    let hash = if write {
        let temp = "temporary";
        let hash = write_blob(
            &file,
            std::fs::File::create("temporary").context("write blob object to temp file")?,
        )?;
        fs::create_dir_all(format!(".git/objects/{:?}", &hash[..2]))
            .context("create sub directory .git/objects")?;

        fs::rename(
            temp,
            format!(".git/objects/{:?}/{:?}", &hash[..2], &hash[2..]),
        )
        .context("move blob file into git objects")?;
        hash
    } else {
        write_blob(&file, std::io::sink()).context("write out blob")?
    };

    println!("{hash}");

    Ok(())
}
