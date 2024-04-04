use std::process::exit;

use clap::{Parser, Subcommand};
use kvs::net::client::KvClient;

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(
        long,
        value_name = "IP-PORT",
        default_value = "127.0.0.1:4000",
        global = true
    )]
    addr: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Set { key: String, value: String },
    Get { key: String },
    Rm { key: String },
}

fn main() {
    let cli = Cli::parse();
    let mut client = KvClient::new(cli.addr.unwrap_or("127.0.0.1:4000".to_string()));
    match &cli.command {
        Commands::Set { key, value } => match client.set(key.to_owned(), value.to_owned()) {
            Ok(()) => {}
            Err(e) => fail(e),
        },
        Commands::Get { key } => match client.get(key.to_owned()) {
            Ok(Some(value)) => println!("{}", value),
            Ok(None) => println!("Key not found"),
            Err(e) => fail(e),
        },
        Commands::Rm { key } => match client.remove(key.to_owned()) {
            Ok(()) => {}
            Err(e) => fail(e),
        },
    }
}

fn fail(reason: String) -> ! {
    eprintln!("{}", reason);
    exit(1);
}
