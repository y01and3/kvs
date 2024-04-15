pub mod kvs;
pub mod net;
pub mod thread_pool;

pub use kvs::kv_store::KvStore;
pub use kvs::kvs_engine::{KvsEngine, SledKvsEngine};
use serde::{de::DeserializeOwned, Serialize};

pub type Result<T> = std::result::Result<T, String>;

pub trait ToString {
    fn to_string(&self) -> String;
}

pub trait ToResult {
    fn to_result<T: DeserializeOwned + Clone>(&self) -> Result<T>;
}

impl<T: Serialize> ToString for Result<T> {
    fn to_string(&self) -> String {
        match self {
            Ok(i) => format!("Ok {}", serde_json::to_string(i).unwrap()),
            Err(e) => format!("Err {}", e),
        }
    }
}

impl ToResult for String {
    fn to_result<T: DeserializeOwned + Clone>(&self) -> Result<T> {
        if self.is_empty() {
            return Err("Empty result".to_string());
        }
        let mut parts = self.splitn(2, ' ');
        let result = parts.next().unwrap();
        let value = parts.next().unwrap();
        match result {
            "Ok" => Ok(serde_json::from_str::<T>(value).map_err(|e| e.to_string())?),
            "Err" => Err(value.to_string()),
            _ => Err("Invalid result".to_string()),
        }
    }
}
