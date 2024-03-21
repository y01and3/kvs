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

pub type Result<T> = std::result::Result<T, String>;

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

    /// Set the value of a string key to a string.
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        catch_unwind!(
            {
                self.map.insert(key.clone(), value.clone());
                self.store();
                self.log(format!("set({}, {})\n", key, value));
            },
            "The value is not written successfully."
        )
    }

    /// Get the string value of a string key. If the key does not exist, return None.
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
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
    pub fn remove(&mut self, key: String) -> Result<()> {
        match catch_unwind!(
            {
                let result = self.map.remove(key.as_str());
                self.store();
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
    /// Open a KvStore at a given path.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        catch_unwind!(
            {
                let path = path.into();
                let mut fs = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path.join("kvs.db"))
                    .unwrap();
                let mut buf = String::new();
                let log = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path.with_extension("log"))
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

    /// Store the map to the file.
    fn store(&mut self) {
        match &self.file {
            Some(fs) => {
                let mut fs = fs;
                let buf = serde_json::to_string(&self.map).unwrap();
                fs.set_len(0).unwrap();
                fs.seek(SeekFrom::Start(0)).unwrap();
                fs.write_all(buf.as_bytes()).unwrap();
            }
            None => {}
        }
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
