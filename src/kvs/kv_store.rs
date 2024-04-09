use super::kvs_engine::KvsEngine;
use crate::Result;
use log::trace;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

/// The `KvStore` stores string key/value pairs.
pub struct KvStore {
    map: HashMap<String, String>,
    file: File,
}

impl KvStore {
    /// Create a new `KvStore`
    pub fn new() -> KvStore {
        KvStore {
            map: HashMap::new(),
            file: File::create("store").unwrap(),
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
                Ok(_) => serde_json::from_str(buf.as_str()).unwrap_or(HashMap::new()),
                Err(_) => HashMap::new(),
            },
            file: fs,
        })
    }

    /// Store the map to the file.
    fn store(&mut self) -> Result<()> {
        let mut fs = self.file.try_clone().map_err(|e| e.to_string())?;
        let buf = serde_json::to_string(&self.map).map_err(|e| e.to_string())?;
        fs.set_len(0).map_err(|e| e.to_string())?;
        fs.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        fs.write_all(buf.as_bytes()).map_err(|e| e.to_string())?;

        Ok(())
    }
}

impl KvsEngine for KvStore {
    /// Set the value of a string key to a string.
    fn set(&mut self, key: String, value: String) -> Result<()> {
        self.map.insert(key.clone(), value.clone());
        self.store()?;
        trace!("set:\t{}", key);
        Ok(())
    }

    /// Get the string value of a string key. If the key does not exist, return None.
    fn get(&mut self, key: String) -> Result<Option<String>> {
        let value = self
            .map
            .get_key_value(key.as_str())
            .map(|(_, v)| v.to_string());
        trace!("get:\t{}", key);
        Ok(value)
    }

    /// Remove a key.
    fn remove(&mut self, key: String) -> Result<()> {
        let result = self.map.remove(key.as_str());
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
