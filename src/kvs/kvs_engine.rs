use crate::Result;
use sled::Db;
pub trait KvsEngine {
    fn set(&mut self, key: String, value: String) -> Result<()>;
    fn get(&mut self, key: String) -> Result<Option<String>>;
    fn remove(&mut self, key: String) -> Result<()>;
    fn store(&mut self) -> Result<()>;
}

pub struct SledKvsEngine {
    map: Db,
}

impl SledKvsEngine {
    pub fn new(map: Db) -> SledKvsEngine {
        SledKvsEngine { map }
    }
}

impl KvsEngine for SledKvsEngine {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        self.map
            .insert(key.as_bytes(), value.as_bytes())
            .map_err(|e| e.to_string())?;
        self.store()?;
        Ok(())
    }

    fn get(&mut self, key: String) -> Result<Option<String>> {
        match self.map.get(key.as_bytes()).map_err(|e| e.to_string())? {
            Some(value) => Ok(Some(
                String::from_utf8(value.to_vec()).map_err(|e| e.to_string())?,
            )),
            None => Ok(None),
        }
    }

    fn remove(&mut self, key: String) -> Result<()> {
        self.map.remove(key.as_bytes()).map_or_else(
            |e| Err(e.to_string()),
            |result| {
                if result.is_none() {
                    Err("Key not found".to_string())
                } else {
                    self.store()?;
                    Ok(())
                }
            },
        )
    }

    fn store(&mut self) -> Result<()> {
        self.map.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl Drop for SledKvsEngine {
    fn drop(&mut self) {
        self.store().unwrap();
    }
}
