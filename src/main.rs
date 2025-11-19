// src/main.rs

mod cache;
mod cluster;
mod config;
mod error;
mod http_server;
mod rpc_client;
mod rpc_server;
mod logger;

#[allow(unused_imports)]
use crate::{
    cache::{CacheStore, SharedCache},
    cluster::{Cluster, SharedCluster},
    config::{Settings, SharedSettings},
};
use std::sync::Arc;
use log::{info, error, debug};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    // tracing_subscriber::fmt::init();

    let settings = match Settings::new() {
        Ok(s) => Arc::new(s),
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(e.into());
        }
    };

    logger::init_logger(&settings.log_level);
    info!("Starting MyCache Node...");
    info!("Configuration loaded successfully.");
    info!("Initialized logger with level: [{}]", settings.log_level);
    debug!("Settings: {:?}", settings);

    let cache: SharedCache = Arc::new(CacheStore::new(
        settings.cache.capacity,
        settings.cache.default_ttl_seconds,
    ));
    info!("Cache store initialized.");

    let cluster: SharedCluster = Arc::new(Cluster::new(&settings));
    info!("Cluster ring initialized.");

    let settings_rpc = Arc::clone(&settings);
    let cache_rpc = Arc::clone(&cache);
    let rpc_handle =
        tokio::spawn(async move { rpc_server::run_rpc_server(settings_rpc, cache_rpc).await });
    info!("Spawned gRPC server task.");

    let settings_http = Arc::clone(&settings);
    let cache_http = Arc::clone(&cache);
    let cluster_http = Arc::clone(&cluster);
    let http_handle = tokio::spawn(async move {
        http_server::run_http_server(settings_http, cache_http, cluster_http).await
    });
    info!("Spawned HTTP server task.");

    tokio::select! {
        res = rpc_handle => {
            match res {
                Ok(Err(e)) => error!("gRPC server task failed: {}", e),
                Ok(Ok(())) => error!("gRPC server task exited successfully (which it shouldn't)"),
                Err(e) => error!("gRPC server task panicked: {}", e),
            }
        }
        
        // 等待 HTTP 任务的结果
        res = http_handle => {
            match res {
                Ok(Err(e)) => error!("HTTP server task failed: {}", e),
                Ok(Ok(())) => error!("HTTP server task exited successfully (which it shouldn't)"),
                Err(e) => error!("HTTP server task panicked: {}", e),
            }
        }
    }

    error!("A critical service task failed. Shutting down.");
    Ok(())
}
