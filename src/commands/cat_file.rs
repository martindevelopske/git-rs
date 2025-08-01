#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;

use anyhow::Context;

use crate::objects::Kind;
use crate::objects::Object;

pub fn invoke(pretty_print: bool, object_hash: String) -> anyhow::Result<()> {
    anyhow::ensure!(
        pretty_print,
        "mode must be given without -p, and we dont support mode."
    );

    let mut object = Object::read(&object_hash).context("parse out blob object file")?;
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    match object.kind {
        Kind::Blob => {
            let n = std::io::copy(&mut object.reader, &mut stdout)
                .context("write git object file to stdout")?;

            anyhow::ensure!(
                n == object.expected_size,
                "git/objects file was not the expected size: expected: {}, actual {n}",
                object.expected_size
            );
        }
        _ => anyhow::bail!("I do not yet know how to print {}", object.kind),
    }
    Ok(())
}
