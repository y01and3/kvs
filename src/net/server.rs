use crate::{KvsEngine, Result, ToString};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
};

pub struct KvServer<T: KvsEngine> {
    pub store: Arc<Mutex<T>>,
    listener: TcpListener,
}

impl<T: KvsEngine> KvServer<T> {
    pub fn new(store: T, addr: String) -> KvServer<T> {
        KvServer {
            store: Arc::new(Mutex::new(store)),
            listener: TcpListener::bind(addr.to_string())
                .map_err(|e| e.to_string())
                .unwrap(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let listener = self.listener.try_clone().map_err(|e| e.to_string())?;
        for stream in listener.incoming() {
            let mut stream = stream.map_err(|e| e.to_string())?;
            let mut buf = [0; 512];
            let len = stream.read(&mut buf).map_err(|e| e.to_string())?;
            let request = String::from_utf8(buf[0..len].to_vec()).map_err(|e| e.to_string())?;
            let response = self.handle_request(request);
            stream
                .write(response.as_bytes())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn handle_request(&mut self, request: String) -> String {
        let request = request.trim();
        let mut parts = request.splitn(2, ' ');
        let command = parts.next().unwrap();
        let key = parts.next().unwrap();
        match command {
            "GET" => self.store.lock().unwrap().get(key.to_string()).to_string(),
            "SET" => {
                let kv: Vec<&str> = key.split(' ').collect();
                if kv.len() != 2 {
                    return "Invalid input".to_string();
                }
                self.store
                    .lock()
                    .unwrap()
                    .set(kv[0].to_string(), kv[1].to_string())
                    .to_string()
            }
            "REMOVE" => self
                .store
                .lock()
                .unwrap()
                .remove(key.to_string())
                .to_string(),
            _ => "Invalid command".to_string(),
        }
    }
}
