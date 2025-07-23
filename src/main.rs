use std::any;
#[allow(unused_imports)]
use std::env;
use std::ffi::CStr;
#[allow(unused_imports)]
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;

use anyhow::Context;
use clap::command;
use clap::Parser;
use clap::Subcommand;
use flate2::write;

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
}

enum Kind {
    Blob,
    // Commit
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
            println!("Initialized git directory")
        }
        Command::CatFile {
            pretty_print: _,
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
        }
    }

    Ok(())
}
