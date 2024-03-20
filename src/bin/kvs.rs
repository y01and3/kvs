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
            match store.set(key.to_owned(), value.to_owned()){
                Ok(()) => {}
                Err(e) => fail(e),
            }
        }
        Commands::Get { key } => {
            match store.get(key.to_owned()) {
                Ok(Some(value)) => println!("{}", value),
                Ok(None) => println!("Key not found"),
                Err(e) => fail(e),
            }
        }
        Commands::Rm { key } => {
            match store.remove(key.to_owned()) {
                Ok(()) => {}
                Err(e) => {
                    if e == "Key not found" {
                        println!("{}", e);
                        exit(1);
                    }
                    else {
                        fail(e);
                    }
                },
            } 
        }
    }
}

fn fail(reason: String) -> ! {
    eprintln!("{}", reason);
    exit(1);
}
