// src/rpc_client.rs

use crate::cache::CacheItemTTL;
use crate::error::RpcClientError;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tonic::transport::{Channel, Endpoint};
use log::{warn, info};

pub mod proto_cache {
    tonic::include_proto!("my.cache");
}

use proto_cache::{
    cache_service_client::CacheServiceClient, 
    set_request::TtlOption, 
    DeleteRequest, GetRequest, SetRequest,
};

#[derive(Debug, Clone)]
pub struct RpcClient {
    pool: Arc<DashMap<String, CacheServiceClient<Channel>>>,
    
    connect_timeout: Duration,
}

impl RpcClient {
    pub fn new(connect_timeout_ms: u64) -> Self {
        Self {
            pool: Arc::new(DashMap::new()),
            connect_timeout: Duration::from_millis(connect_timeout_ms),
        }
    }

    async fn get_client(&self, target_addr: &str) -> Result<CacheServiceClient<Channel>, RpcClientError> {
        if let Some(client) = self.pool.get(target_addr) {
            info!("Reusing existing gRPC connection for {}", target_addr);
            return Ok(client.clone());
        }

        info!("Creating new gRPC connection for {}", target_addr);
        
        let channel = Endpoint::from_shared(target_addr.to_string())?
            .connect_timeout(self.connect_timeout)
            .connect()
            .await?;

        let client = CacheServiceClient::new(channel);

        self.pool.insert(target_addr.to_string(), client.clone());
        Ok(client)
    }

    pub async fn forward_get(&self, key: &str, target_addr: &str) -> Result<Value, RpcClientError> {
        let mut client = self.get_client(target_addr).await?;
        
        let request = tonic::Request::new(GetRequest {
            key: key.to_string(),
        });

        match client.internal_get(request).await {
            Ok(response) => {
                let value_json = response.into_inner().value_json;
                let value: Value = serde_json::from_str(&value_json)?;
                Ok(value)
            }
            Err(status) => {
                warn!("gRPC forward_get failed for key {}: {}", key, status);
                if status.code() == tonic::Code::Unavailable {
                    self.pool.remove(target_addr);
                }
                Err(status.into()) 
            }
        }
    }

    pub async fn forward_set(&self, key: String, value: Value, ttl: CacheItemTTL, target_addr: &str) -> Result<(), RpcClientError> {
        let mut client = self.get_client(target_addr).await?;

        let ttl_option = match ttl {
            CacheItemTTL::Default => TtlOption::UseDefaultTtl(true),
            CacheItemTTL::Permanent => TtlOption::SetPermanent(true),
            CacheItemTTL::Custom(d) => TtlOption::SpecificTtlSeconds(d.as_secs()),
        };

        let value_json = serde_json::to_string(&value)?;

        let request = tonic::Request::new(SetRequest {
            key,
            value_json,
            ttl_option: Some(ttl_option),
        });

        match client.internal_set(request).await {
            Ok(_) => Ok(()),
            Err(status) => {
                warn!("gRPC forward_set failed: {}", status);
                if status.code() == tonic::Code::Unavailable {
                    self.pool.remove(target_addr);
                }
                Err(status.into())
            }
        }
    }

    pub async fn forward_delete(&self, key: &str, target_addr: &str) -> Result<i64, RpcClientError> {
        let mut client = self.get_client(target_addr).await?;

        let request = tonic::Request::new(DeleteRequest {
            key: key.to_string(),
        });

        match client.internal_delete(request).await {
            Ok(response) => Ok(response.into_inner().deleted_count),
            Err(status) => {
                warn!("gRPC forward_delete failed for key {}: {}", key, status);
                if status.code() == tonic::Code::Unavailable {
                    self.pool.remove(target_addr);
                }
                Err(status.into())
            }
        }
    }
}