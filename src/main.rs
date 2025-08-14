#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::path::PathBuf;

use clap::command;
use clap::Parser;
use clap::Subcommand;

mod commands;
pub(crate) mod objects;
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
    LsTree {
        #[clap(long)]
        name_only: bool,

        tree_hash: String,
    },
    WriteTree,
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    let args = Args::parse();
    match args.command {
        Command::Init => commands::init::invoke(),
        Command::CatFile {
            pretty_print,
            object_hash,
        } => commands::cat_file::invoke(pretty_print, object_hash),
        Command::HashObject { write, file } => commands::hash_object::invoke(write, file),

        Command::LsTree {
            name_only,
            tree_hash,
        } => commands::ls_tree::invoke(name_only, &tree_hash),
        Command::WriteTree => commands::write_tree::invoke(),
    }
}
