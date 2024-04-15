use crate::{thread_pool::ThreadPool, KvsEngine, Result, ToString};
use std::{
    io::{Read, Write},
    net::TcpListener,
};

pub struct KvServer<T: KvsEngine, R: ThreadPool> {
    store: T,
    listener: TcpListener,
    thread_pool: R,
}

impl<T: KvsEngine, R: ThreadPool> KvServer<T, R> {
    pub fn new(store: T, addr: String, thread_pool: R) -> KvServer<T, R> {
        KvServer {
            store,
            listener: TcpListener::bind(addr.to_string())
                .map_err(|e| e.to_string())
                .unwrap(),
            thread_pool,
        }
    }

    pub fn run(&self) -> Result<()> {
        let listener = self.listener.try_clone().map_err(|e| e.to_string())?;
        for stream in listener.incoming() {
            let store = self.store.clone();
            self.thread_pool.spawn(move || match stream {
                Ok(stream) => {
                    if let Err(e) = handle_connection(&store, stream) {
                        eprintln!("Error on serving client: {}", e);
                    }
                }
                Err(e) => eprintln!("Connection failed: {}", e),
            });
        }
        Ok(())
    }
}

fn handle_connection<T: KvsEngine>(store: &T, mut stream: std::net::TcpStream) -> Result<()> {
    let mut buf = [0; 512];
    let len = stream.read(&mut buf).map_err(|e| e.to_string())?;
    let request = String::from_utf8(buf[0..len].to_vec()).map_err(|e| e.to_string())?;
    let response = handle_request(store, request);
    stream
        .write(response.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn handle_request<T: KvsEngine>(store: &T, request: String) -> String {
    let request = request.trim();
    let mut parts = request.splitn(2, ' ');
    let command = parts.next().unwrap();
    let key = parts.next().unwrap();
    match command {
        "GET" => store.get(key.to_string()).to_string(),
        "SET" => {
            let kv: Vec<&str> = key.split(' ').collect();
            if kv.len() != 2 {
                return "Invalid input".to_string();
            }
            store.set(kv[0].to_string(), kv[1].to_string()).to_string()
        }
        "REMOVE" => store.remove(key.to_string()).to_string(),
        _ => "Invalid command".to_string(),
    }
}
