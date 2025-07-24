use flate2::write::ZlibEncoder;
use flate2::Compression;
use hex::encode;
use sha1::{Digest, Sha1};
#[allow(unused_imports)]
use std::env;
use std::ffi::CStr;
#[allow(unused_imports)]
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use clap::command;
use clap::Parser;
use clap::Subcommand;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// initialize a new git repo
    Init,

    /// read the contents of a blob
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,
        object_hash: String,
    },
    HashObject {
        #[clap(short = 'w')]
        write: bool,

        file: PathBuf,
    },
}

enum Kind {
    Blob,
    // Commit
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

fn main() -> anyhow::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    let args = Args::parse();
    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory");

            Ok(())
        }
        Command::CatFile {
            pretty_print,
            object_hash,
        } => {
            anyhow::ensure!(
                pretty_print,
                "mode must be given without -p, and we dont support mode."
            );

            let f = std::fs::File::open(format!(
                ".git/objects/{}/{}",
                &object_hash[..2],
                &object_hash[2..]
            ))
            .context("open in .git/objects")?;
            //decompress using flate2
            let decoder = flate2::read::ZlibDecoder::new(f);
            let mut decoder = BufReader::new(decoder);
            let mut buf = Vec::new();
            decoder
                .read_until(0, &mut buf)
                .context("read header from .git/objects")?;

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
                _ => anyhow::bail!("do not yet know how to print a '{kind}'"),
            };

            let size = size
                .parse::<usize>()
                .context(".git/objects file header has invalid size: '{size}'")?;

            buf.clear();
            buf.resize(size, 0);

            decoder
                .read_exact(&mut buf[..])
                .context("read true contents of .git/objects file")?;

            let n = decoder
                .read(&mut [0])
                .context("valid Eof in .git/objects file")?;

            anyhow::ensure!(n == 0, ".git/objects file had trailing bytes");

            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();

            match kind {
                Kind::Blob => stdout
                    .write_all(&buf)
                    .context("write object contents to stdout")?,
            }
            Ok(())
        }
        Command::HashObject { write, file } => {
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
                let stat =
                    std::fs::metadata(file).with_context(|| format!("stat {}", file.display()))?;
                let writer = ZlibEncoder::new(writer, Compression::default());
                let mut writer = HashWriter {
                    writer,
                    hasher: Sha1::new(),
                };

                write!(writer, "blob ");
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
    }
}
