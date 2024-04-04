use std::{
    io::{Read, Write},
    net::TcpStream,
};

use crate::{Result, ToResult};

pub struct KvClient {
    stream: TcpStream,
}

impl KvClient {
    pub fn new(addr: String) -> KvClient {
        KvClient {
            stream: TcpStream::connect(addr).unwrap(),
        }
    }

    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        self.stream
            .write_all(format!("GET {}\n", key).as_bytes())
            .map_err(|e| e.to_string())?;
        let mut buf = String::new();
        self.stream
            .read_to_string(&mut buf)
            .map_err(|e| e.to_string())?;
        buf.to_result()
    }

    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        self.stream
            .write_all(format!("SET {} {}\n", key, value).as_bytes())
            .map_err(|e| e.to_string())?;
        let mut buf = String::new();
        self.stream
            .read_to_string(&mut buf)
            .map_err(|e| e.to_string())?;
        buf.to_result()
    }

    pub fn remove(&mut self, key: String) -> Result<()> {
        self.stream
            .write_all(format!("REMOVE {}\n", key).as_bytes())
            .map_err(|e| e.to_string())?;
        let mut buf = String::new();
        self.stream
            .read_to_string(&mut buf)
            .map_err(|e| e.to_string())?;
        buf.to_result()
    }
}
