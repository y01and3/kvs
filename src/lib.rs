pub mod kvs;
pub use kvs::kv_store::KvStore;
pub use kvs::kvs_engine::KvsEngine;

pub type Result<T> = std::result::Result<T, String>;