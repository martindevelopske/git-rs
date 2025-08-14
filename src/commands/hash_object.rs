#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;

use crate::objects::Object;

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
        let hash = Object::blob_from_file(file)
            .context("open blob input file")?
            .write(writer)
            .context("Stream file into blob")?;

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
