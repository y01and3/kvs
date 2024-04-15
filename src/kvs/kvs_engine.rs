use std::sync::{Arc, Mutex};

use crate::Result;
use sled::Db;
pub trait KvsEngine: Clone + Send + 'static {
    fn set(&self, key: String, value: String) -> Result<()>;
    fn get(&self, key: String) -> Result<Option<String>>;
    fn remove(&self, key: String) -> Result<()>;
}

#[derive(Clone)]
pub struct SledKvsEngine {
    map: Arc<Mutex<Db>>,
}

impl SledKvsEngine {
    pub fn new(map: Db) -> SledKvsEngine {
        SledKvsEngine {
            map: Arc::new(Mutex::new(map)),
        }
    }

    pub fn store(&self) -> Result<()> {
        loop {
            match self.map.try_lock() {
                Ok(map) => {
                    map.flush().map_err(|e| e.to_string())?;
                    break;
                }
                Err(std::sync::TryLockError::WouldBlock) => continue,
                Err(_) => panic!("Poisoned lock"),
            }
        }
        Ok(())
    }
}

impl KvsEngine for SledKvsEngine {
    fn set(&self, key: String, value: String) -> Result<()> {
        loop {
            match self.map.try_lock() {
                Ok(map) => {
                    map.insert(key.as_bytes(), value.as_bytes())
                        .map_err(|e| e.to_string())?;
                    break;
                }
                Err(std::sync::TryLockError::WouldBlock) => continue,
                Err(_) => panic!("Poisoned lock"),
            }
        }
        self.store()?;
        Ok(())
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        loop {
            match self.map.try_lock() {
                Ok(map) => {
                    break match map.get(key.as_bytes()).map_err(|e| e.to_string())? {
                        Some(value) => Ok(Some(
                            String::from_utf8(value.to_vec()).map_err(|e| e.to_string())?,
                        )),
                        None => Ok(None),
                    }
                }
                Err(std::sync::TryLockError::WouldBlock) => continue,
                Err(_) => panic!("Poisoned lock"),
            }
        }
    }

    fn remove(&self, key: String) -> Result<()> {
        let result = loop {
            match self.map.try_lock() {
                Ok(map) => break map.remove(key.as_bytes()).map_err(|e| e.to_string())?,
                Err(std::sync::TryLockError::WouldBlock) => continue,
                Err(_) => panic!("Poisoned lock"),
            }
        };
        self.store()?;
        match result {
            Some(_) => Ok(()),
            None => Err("Key not found".to_string()),
        }
    }
}

impl Drop for SledKvsEngine {
    fn drop(&mut self) {
        self.store().unwrap();
    }
}
