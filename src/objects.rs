use anyhow::Context;
use core::fmt;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest, Sha1};
#[allow(unused_imports)]
use std::env;
use std::ffi::CStr;
#[allow(unused_imports)]
use std::fs;
use std::io::prelude::*;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Blob,
    Commit,
    Tree,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Blob => write!(f, "blob"),
            Kind::Commit => write!(f, "commit"),
            Kind::Tree => write!(f, "tree"),
        }
    }
}

pub(crate) struct Object<R> {
    pub(crate) kind: Kind,
    pub(crate) expected_size: u64,
    pub(crate) reader: R,
}

/// object format for any git object:
/// <type> <size>\0<content>
///
/// also git encodes the entire objects before hashing and storing them
///
///what this function does:
/// 1. extract the object kind i.e blob, commit, tree
/// 2. extract the size of the object
/// 3. give out a reader that will be used to read the content of the object
impl Object<()> {
    pub(crate) fn read(hash: &str) -> anyhow::Result<Object<impl BufRead>> {
        let f = std::fs::File::open(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))
            .context("open in .git/objects")?;

        //decompress using flate2
        let decoder = flate2::read::ZlibDecoder::new(f);
        let mut decoder = BufReader::new(decoder);
        let mut buf = Vec::new();
        decoder
            .read_until(0, &mut buf)
            .context("read header from .git/objects")?;

        // this header contains the type and size
        let header = CStr::from_bytes_with_nul(&buf)
            .expect("know there is exactly one nul, and it's at the end.");

        let header = header
            .to_str()
            .context(".git/objects file header is not valid UTF-8")?;

        // println!("the header is: {}", header);

        let Some((kind, size)) = header.split_once(' ') else {
            anyhow::bail!(".git/objects file header did not start with 'blob': '{header}'")
        };

        let kind = match kind {
            "blob" => Kind::Blob,
            "commit" => Kind::Commit,
            "tree" => Kind::Tree,
            _ => anyhow::bail!("do not yet know how to print a '{kind}'"),
        };

        let size = size
            .parse::<u64>()
            .context(".git/objects file header has invalid size: '{size}'")?;

        buf.clear();

        //resize it to the size of the remaining content
        // buf.resize(size, 0);

        // decoder
        //     .read_exact(&mut buf[..])
        //     .context("read true contents of .git/objects file")?;
        //
        // let n = decoder
        //     .read(&mut [0])
        //     .context("valid Eof in .git/objects file")?;
        //
        // anyhow::ensure!(n == 0, ".git/objects file had trailing bytes");
        //
        // let stdout = std::io::stdout();
        // let mut stdout = stdout.lock();
        //
        Ok(Object {
            kind,
            expected_size: size,
            reader: decoder,
        })
    }

    pub(crate) fn blob_from_file(file: impl AsRef<Path>) -> anyhow::Result<Object<impl Read>> {
        let file = file.as_ref();
        let stat = std::fs::metadata(file).with_context(|| format!("stat {}", file.display()))?;
        // TODO: there is a potential race here if the file changes between stat and write
        //
        let file_reader =
            std::fs::File::open(file).with_context(|| format!("open file {}", file.display()))?;
        let reader = BufReader::new(file_reader);

        Ok(Object {
            kind: Kind::Blob,
            expected_size: stat.len(),
            reader,
        })
    }
}

impl<R> Object<R>
where
    R: Read,
{
    // read from the self reader into the the provided writer.
    pub(crate) fn write(mut self, writer: impl Write) -> anyhow::Result<[u8; 20]> {
        let writer = ZlibEncoder::new(writer, Compression::default());
        let mut writer = HashWriter {
            writer,
            hasher: Sha1::new(),
        };

        write!(writer, "{} {}\0", self.kind, self.expected_size)?;
        std::io::copy(&mut self.reader, &mut writer).context("stream file into blob")?;
        let _ = writer.writer.finish();
        let hash = writer.hasher.finalize();
        Ok(hash.into())
    }
    pub(crate) fn write_to_objects(self) -> anyhow::Result<[u8; 20]> {
        let tmp = "temporary";
        let hash = self
            .write(std::fs::File::create(tmp).context("construct temporary file for tree")?)
            .context("stream tree object into tree object file")?;
        let hash_hex = hex::encode(hash);
        fs::create_dir_all(format!(".git/objects/{}/", &hash_hex[..2]))
            .context("create subdir of .git/objects")?;
        fs::rename(
            tmp,
            format!(".git/objects/{}/{}", &hash_hex[..2], &hash_hex[2..]),
        )
        .context("move tree file into .git/objects")?;
        Ok(hash)
    }
}

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
