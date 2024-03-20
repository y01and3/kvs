use std::process::exit;

use clap::{Parser, Subcommand};
use kvs::KvStore;

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Set { key: String, value: String },
    Get { key: String },
    Rm { key: String },
}

fn main() {
    let mut store = KvStore::new();
    let cli = Cli::parse();
    match &cli.command {
        Commands::Set { key, value } => {
            store.set(key.to_owned(), value.to_owned());
            eprintln!("unimplemented");
            exit(1);
        }
        Commands::Get { key: _ } => {
            // match store.get(key.to_owned()) {
            //     Some(value) => println!("{}", value),
            //     None => println!("Key not found"),
            // }

            eprintln!("unimplemented");
            exit(1);
        }
        Commands::Rm { key } => {
            store.remove(key.to_owned());
            eprintln!("unimplemented");
            exit(1);
        }
    }
}
