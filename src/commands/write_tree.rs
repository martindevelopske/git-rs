use std::{cmp::Ordering, fs, io::Cursor, os::unix::fs::PermissionsExt, path::Path};

use anyhow::Context;
use log::debug;

use crate::objects::{Kind, Object};

/// process
/// 1. traverse the directory recursively
///    - for each file, create blob object
///    - for each sub dir, recursively call write-tree
/// 2. build the tree object
/// 3. create the final tree object
pub(crate) fn invoke() -> anyhow::Result<()> {
    debug!("Starting write-tree from root '.'");
    let Some(hash) = write_tree_for(Path::new(".")).context("construct root tree object")? else {
        anyhow::bail!("asked to make tree object for empty tree");
    };

    println!("{}", hex::encode(hash));
    debug!("Finished write-tree for root. Hash: {}", hex::encode(hash));

    Ok(())
}

fn write_tree_for(path: &Path) -> anyhow::Result<Option<[u8; 20]>> {
    debug!("Entering directory: {}", path.display());
    let dir = fs::read_dir(path).with_context(|| format!("open directory {}", path.display()))?;

    let mut entries = Vec::new();
    for entry in dir {
        let entry = entry.with_context(|| format!("bad directory entry in {}", path.display()))?;
        let meta = entry.metadata().context("metadata for directory entry")?;
        let file_name = entry.file_name();
        entries.push((entry, file_name, meta));
    }

    entries.sort_unstable_by(|a, b| {
        // git has very specific rules for how to compare names
        // https://github.com/git/git/blob/e09f1254c54329773904fe25d7c545a1fb4fa920/tree.c#L99
        let afn = &a.1;
        let afn = afn.as_encoded_bytes();
        let bfn = &b.1;
        let bfn = bfn.as_encoded_bytes();
        let common_len = std::cmp::min(afn.len(), bfn.len());
        match afn[..common_len].cmp(&bfn[..common_len]) {
            Ordering::Equal => {}
            o => return o,
        }
        if afn.len() == bfn.len() {
            return Ordering::Equal;
        }
        let c1 = if let Some(c) = afn.get(common_len).copied() {
            Some(c)
        } else if a.2.is_dir() {
            Some(b'/')
        } else {
            None
        };
        let c2 = if let Some(c) = bfn.get(common_len).copied() {
            Some(c)
        } else if b.2.is_dir() {
            Some(b'/')
        } else {
            None
        };

        c1.cmp(&c2)
    });

    let mut tree_object = Vec::new();
    for (entry, file_name, meta) in entries {
        let file_name_str = file_name.to_string_lossy();
        debug!("Found entry: {}", file_name_str);

        if file_name_str == ".git"
            || file_name_str == "."
            || file_name_str == ".."
            || file_name_str == "target"
        {
            debug!("Skipping reserved path: {}", file_name_str);
            continue;
        }
        //
        // if path.components().any(|c| c.as_os_str() == ".git") {
        //     debug!("Skipping nested .git path: {}", path.display());
        //     continue;
        // }

        let mode = if meta.is_dir() {
            "40000"
        } else if meta.is_symlink() {
            "120000"
        } else if (meta.permissions().mode() & 0o111) != 0 {
            "100755"
        } else {
            "100644"
        };
        debug!("Mode for {}: {}", file_name_str, mode);
        let path = entry.path();

        let hash = if meta.is_dir() {
            debug!("Recursing into subdirectory: {}", path.display());
            let Some(hash) = write_tree_for(&path)? else {
                debug!("Skipping empty directory: {}", path.display());
                continue;
            };
            hash
        } else {
            debug!("Creating blob for file: {}", path.display());
            let tmp = "temporary";
            let hash = Object::blob_from_file(&path)
                .context("open blob input file")?
                .write(std::fs::File::create(tmp).context("construct temporary file for blob")?)
                .context("stream file into blob")?;
            let hash_hex = hex::encode(hash);
            debug!("Blob hash for {}: {}", path.display(), hash_hex);

            fs::create_dir_all(format!(".git/objects/{}/", &hash_hex[..2]))
                .context("create subdir of .git/objects")?;
            std::fs::rename(
                tmp,
                format!(".git/objects/{}/{}", &hash_hex[..2], &hash_hex[2..]),
            )
            .context("move blob file into .git/objects")?;
            hash
        };

        tree_object.extend(mode.as_bytes());
        tree_object.push(b' ');
        tree_object.extend(file_name.as_encoded_bytes());
        tree_object.push(0);
        tree_object.extend(hash);
        debug!("Added tree entry: {} {}", mode, file_name_str);
    }

    if tree_object.is_empty() {
        debug!("Directory {} was empty", path.display());
        Ok(None)
    } else {
        debug!("Creating tree object for directory: {}", path.display());
        Ok(Some(
            Object {
                kind: Kind::Tree,
                expected_size: tree_object.len() as u64,
                reader: Cursor::new(tree_object),
            }
            .write_to_objects()
            .context("write tree object")?,
        ))
    }
}
