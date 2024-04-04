use super::kvs_engine::KvsEngine;
use crate::Result;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    panic,
    path::PathBuf,
};

/// The `KvStore` stores string key/value pairs.
pub struct KvStore {
    map: HashMap<String, String>,
    file: Option<File>,
    log: Option<File>,
}

/// Catch the panic and return the error message.
macro_rules! catch_unwind {
    ($block:block, $err:literal) => {{
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| $block));
        match result {
            Ok(t) => Ok(t),
            Err(_) => Err(($err).to_string()),
        }
    }};
}

impl KvStore {
    /// Create a new `KvStore`
    pub fn new() -> KvStore {
        KvStore {
            map: HashMap::new(),
            file: None,
            log: None,
        }
    }

    /// Open a KvStore at a given path.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        catch_unwind!(
            {
                let path = path.into();
                if !path.exists() {
                    std::fs::create_dir_all(&path).unwrap();
                }
                let mut fs = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path.join("store"))
                    .unwrap();
                let mut buf = String::new();
                let log = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path.join("log"))
                    .unwrap();

                KvStore {
                    map: match fs.read_to_string(&mut buf) {
                        Ok(_) => serde_json::from_str(buf.as_str()).unwrap_or(HashMap::new()),
                        Err(_) => HashMap::new(),
                    },
                    file: Some(fs),
                    log: Some(log),
                }
            },
            "painc"
        )
    }

    /// Log the command to the log file.
    fn log(&mut self, cmd: String) {
        match &self.log {
            Some(fs) => {
                let mut fs = fs;
                fs.write_all((cmd + "\n").as_bytes()).unwrap();
            }
            None => {}
        }
    }
}

impl KvsEngine for KvStore {
    /// Set the value of a string key to a string.
    fn set(&mut self, key: String, value: String) -> Result<()> {
        catch_unwind!(
            {
                self.map.insert(key.clone(), value.clone());
                self.store().unwrap();
                self.log(format!("set({}, {})\n", key, value));
            },
            "The value is not written successfully."
        )
    }

    /// Get the string value of a string key. If the key does not exist, return None.
    fn get(&mut self, key: String) -> Result<Option<String>> {
        catch_unwind!(
            {
                let value = self
                    .map
                    .get_key_value(key.as_str())
                    .map(|(_, v)| v.to_string());
                self.log(format!("get({})\n", key));
                value
            },
            "The value is not read successfully."
        )
    }

    /// Remove a key.
    fn remove(&mut self, key: String) -> Result<()> {
        match catch_unwind!(
            {
                let result = self.map.remove(key.as_str());
                self.store().unwrap();
                self.log(format!("remove({})\n", key));
                match result {
                    Some(_) => Ok(()),
                    None => Err("Key not found".to_string()),
                }
            },
            "The key is not removed successfully."
        ) {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(e),
        }
    }

    /// Store the map to the file.
    fn store(&mut self) -> Result<()> {
        match &self.file {
            Some(fs) => {
                let mut fs = fs;
                let buf = serde_json::to_string(&self.map).map_err(|e| e.to_string())?;
                fs.set_len(0).map_err(|e| e.to_string())?;
                fs.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
                fs.write_all(buf.as_bytes()).map_err(|e| e.to_string())?;
            }
            None => {}
        };
        Ok(())
    }
}

impl Drop for KvStore {
    fn drop(&mut self) {
        self.store().unwrap();
    }
}
