// src/config.rs

use config::{Config, ConfigError, Environment};
use serde::Deserialize;
use std::sync::Arc;
use log::info;

#[derive(Debug, Deserialize, Clone)]
pub struct CacheSettings {
    pub capacity: u64,
    pub default_ttl_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub http_addr: String,
    pub rpc_addr: String,
    pub my_connectable_addr: String,
    pub cluster_nodes: Vec<String>,
    pub cache: CacheSettings,
    pub log_level: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        #[cfg(debug_assertions)]
        {
            dotenv::dotenv().ok();
        }

        for (var, val) in std::env::vars() {
            if var.starts_with("MY_CACHE_") || var.starts_with("SEQ_MY_CACHE_") {
                info!("[Settings] GOT ENV VAR: {} - {}", var, val);
            }
        }

        let s = Config::builder()
            .set_default("http_addr", "0.0.0.0:8000")?
            .set_default("rpc_addr", "0.0.0.0:50051")?
            .set_default("cache.capacity", 10000)?
            .set_default("cache.default_ttl_seconds", 3600)? 
            .set_default("log_level", "info")?

            .add_source(
                Environment::with_prefix("MY_CACHE") 
                    .prefix_separator("_")
                    .separator("__") 
            )
            .add_source(
                Environment::with_prefix("SEQ_MY_CACHE") 
                    .prefix_separator("_")
                    .separator("__")
                    .try_parsing(true)
                    .list_separator(",")
            )
            .build()?;
        s.try_deserialize()
    }
}

pub type SharedSettings = Arc<Settings>;