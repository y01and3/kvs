use std::fs::read_dir;

use clap::Parser;
use kvs::{
    net::server::KvServer,
    thread_pool::{SharedQueueThreadPool, ThreadPool},
    KvStore, SledKvsEngine,
};

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[arg(long, value_name = "IP-PORT", default_value = "127.0.0.1:4000")]
    addr: Option<String>,
    #[arg(long, value_name = "ENGINE-NAME")]
    engine: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    eprintln!("version: {}", env!("CARGO_PKG_VERSION"));
    eprintln!("args: {:?}", std::env::args().collect::<Vec<String>>());
    let mut engine = cli.engine;
    let old_engine = auto_choose_engine();
    if engine.is_some() && old_engine.is_some() && engine != old_engine {
        eprintln!("Engine not match");
        std::process::exit(1);
    } else if engine.is_none() && old_engine.is_some() {
        engine = old_engine;
    }
    match engine.as_deref() {
        Some("kvs") => {
            let sever = KvServer::new(
                KvStore::open("kvs".to_string()).unwrap(),
                cli.addr.unwrap_or("127.0.0.1:4000".to_string()),
                SharedQueueThreadPool::new(4).unwrap(),
            );

            sever.run().unwrap();
        }
        Some("sled") => {
            let sever = KvServer::new(
                SledKvsEngine::new(sled::open("sled").unwrap()),
                cli.addr.unwrap_or("127.0.0.1:4000".to_string()),
                SharedQueueThreadPool::new(4).unwrap(),
            );

            sever.run().unwrap();
        }
        Some(_) => {
            eprintln!("Engine not supported");
            std::process::exit(1);
        }
        None => {
            let sever = KvServer::new(
                KvStore::open("kvs".to_string()).unwrap(),
                cli.addr.unwrap_or("127.0.0.1:4000".to_string()),
                SharedQueueThreadPool::new(4).unwrap(),
            );

            sever.run().unwrap();
        }
    };
}

fn auto_choose_engine() -> Option<String> {
    for entry in read_dir(".").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            let path = path.to_str().unwrap();
            if path.ends_with("sled") {
                return Some("sled".to_string());
            } else if path.ends_with("kvs") {
                return Some("kvs".to_string());
            }
        }
    }
    None
}
