use super::kvs_engine::KvsEngine;
use crate::Result;
use log::trace;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// The `KvStore` stores string key/value pairs.
#[derive(Clone)]
pub struct KvStore {
    map: Arc<Mutex<HashMap<String, String>>>,
    file: Arc<Mutex<File>>,
}

impl KvStore {
    /// Create a new `KvStore`
    pub fn new() -> KvStore {
        KvStore {
            map: Arc::new(Mutex::new(HashMap::new())),
            file: Arc::new(Mutex::new(File::create("store").unwrap())),
        }
    }

    /// Open a KvStore at a given path.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path = path.into();
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        }
        let mut fs = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.join("store"))
            .map_err(|e| e.to_string())?;
        let mut buf = String::new();

        Ok(KvStore {
            map: match fs.read_to_string(&mut buf) {
                Ok(_) => Arc::new(Mutex::new(
                    serde_json::from_str(buf.as_str()).unwrap_or(HashMap::new()),
                )),
                Err(_) => Arc::new(Mutex::new(HashMap::new())),
            },
            file: Arc::new(Mutex::new(fs)),
        })
    }

    /// Store the map to the file.
    fn store(&self) -> Result<()> {
        'outer: loop {
            match self.file.try_lock() {
                Ok(mut fs) => loop {
                    match self.map.try_lock() {
                        Ok(map) => {
                            let buf = serde_json::to_string(&*map).map_err(|e| e.to_string())?;
                            fs.set_len(0).map_err(|e| e.to_string())?;
                            fs.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
                            fs.write_all(buf.as_bytes()).map_err(|e| e.to_string())?;
                            break 'outer;
                        }
                        Err(std::sync::TryLockError::WouldBlock) => continue,
                        Err(_) => panic!("Poisoned lock"),
                    }
                },
                Err(std::sync::TryLockError::WouldBlock) => continue,
                Err(_) => panic!("Poisoned lock"),
            }
        }

        Ok(())
    }
}

impl KvsEngine for KvStore {
    /// Set the value of a string key to a string.
    fn set(&self, key: String, value: String) -> Result<()> {
        loop {
            match self.map.try_lock() {
                Ok(mut map) => {
                    map.insert(key.clone(), value.clone());
                    break;
                }
                Err(std::sync::TryLockError::WouldBlock) => continue,
                Err(_) => panic!("Poisoned lock"),
            }
        }
        self.store()?;
        trace!("set:\t{}", key);
        Ok(())
    }

    /// Get the string value of a string key. If the key does not exist, return None.
    fn get(&self, key: String) -> Result<Option<String>> {
        let value = loop {
            match self.map.try_lock() {
                Ok(map) => break map.get_key_value(key.as_str()).map(|(_, v)| v.to_string()),
                Err(std::sync::TryLockError::WouldBlock) => continue,
                Err(_) => panic!("Poisoned lock"),
            }
        };
        trace!("get:\t{}", key);
        Ok(value)
    }

    /// Remove a key.
    fn remove(&self, key: String) -> Result<()> {
        let result = loop {
            match self.map.try_lock() {
                Ok(mut map) => break map.remove(key.as_str()),
                Err(std::sync::TryLockError::WouldBlock) => continue,
                Err(_) => panic!("Poisoned lock"),
            }
        };
        self.store()?;
        trace!("remove:\t{}", key);
        match result {
            Some(_) => Ok(()),
            None => Err("Key not found".to_string()),
        }
    }
}

impl Drop for KvStore {
    fn drop(&mut self) {
        self.store().unwrap();
    }
}
