pub fn invoke(name_only: bool) -> anyhow::Result<()> {
    // locating the git object(tree) from a ref like HEAD.
    // Decompressing the git object
    // parsing the tree object format
    // printing out the mode, type, sha and file name
    anyhow::ensure!(name_only, "only --name-only is supported for now");
    todo!();
    Ok(())
}
