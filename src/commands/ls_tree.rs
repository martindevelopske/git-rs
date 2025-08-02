use std::{
    ffi::CStr,
    io::{BufRead, Read, Write},
};

use anyhow::Context;

use crate::objects::{Kind, Object};

pub fn invoke(name_only: bool, tree_hash: &str) -> anyhow::Result<()> {
    // locating the git object(tree) from a ref like HEAD.
    // Decompressing the git object
    // parsing the tree object format
    // printing out the mode, type, sha and file name
    let mut object = Object::read(tree_hash).context("parse out tree object file")?;
    match object.kind {
        Kind::Tree => {
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            let mut mode_and_name = Vec::new();
            // let mut sha1_hash = [0u8; 20];

            loop {
                mode_and_name.clear();
                // mode and name
                let n = object
                    .reader
                    .read_until(0, &mut mode_and_name)
                    .context("getting mode and name")?;

                // info!("mode and name is: {:?}", mode_and_name);
                if n == 0 {
                    break;
                }
                let mode_and_name =
                    CStr::from_bytes_with_nul(&mode_and_name).context("invalid tree entry")?;
                // info!("mode and name is: {:?}", mode_and_name);
                let mut bits = mode_and_name.to_bytes().splitn(2, |&x| x == b' ');
                // info!("bits: {:?}", bits);
                let mode = bits.next().expect("split always yields once");
                let name = bits
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("tree entry has no file name"))?;

                //read the 20-byte sha1 hash
                let mut sha1_hash = [0; 20];
                object
                    .reader
                    .read_exact(&mut sha1_hash)
                    .context("reading SHA1 from tree entry")?;

                if name_only {
                    stdout
                        .write_all(name)
                        .context("write tree entry name to stdout")?;
                } else {
                    let mode = std::str::from_utf8(mode).context("mode is always valid utf-8")?;

                    // stdout
                    //     .write_all(mode)
                    //     .context("write tree entry mode to stdout")?;
                    let hash = hex::encode(&sha1_hash);
                    let kind = Object::read(&hash).context("getting the kind")?.kind;
                    write!(stdout, "{mode:0>6} {kind} {hash}    ")
                        .context("write tree entry hash to stdout")?;
                    stdout
                        .write_all(name)
                        .context("write tree entry name to stdout")?;
                }
                writeln!(stdout, "").context("write new line to stdout")?;
            }
        }
        _ => anyhow::bail!("I do not yet know how to ls {}", object.kind),
    }

    Ok(())
}
