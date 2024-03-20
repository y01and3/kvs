use std::collections::HashMap;

/// The `KvStore` stores string key/value pairs.
pub struct KvStore {
    map: HashMap<String, String>,
}

impl KvStore {
    /// Create a new `KvStore`
    pub fn new() -> KvStore {
        KvStore {
            map: HashMap::new(),
        }
    }
    /// Set the value of a string key to a string.
    pub fn set(&mut self, key: String, value: String) {
        self.map.insert(key, value);
    }
    /// Get the string value of a string key. If the key does not exist, return None.
    pub fn get(&self, key: String) -> Option<String> {
        self.map
            .get_key_value(key.as_str())
            .map(|(_, v)| v.to_string())
    }
    /// Remove a key.
    pub fn remove(&mut self, key: String) {
        self.map.remove(key.as_str());
    }
}
