use core::fmt;
#[allow(unused_imports)]
use std::env;
use std::ffi::CStr;
#[allow(unused_imports)]
use std::fs;
use std::io::BufRead;
use std::io::BufReader;

use anyhow::Context;

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
}
