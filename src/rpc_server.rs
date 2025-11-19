// src/rpc_server.rs

use crate::cache::{CacheItemTTL, SharedCache};
use crate::config::SharedSettings;
use serde_json::Value;
use std::net::SocketAddr;
use std::time::Duration;
use tonic::{Request, Response, Status, transport::Server};
use log::{error, info};

pub mod proto_cache {
    tonic::include_proto!("my.cache");
}

use proto_cache::{
    DeleteRequest, DeleteResponse, GetRequest, GetResponse, SetRequest, SetResponse,
    cache_service_server::{CacheService, CacheServiceServer},
};

pub struct MyCacheService {
    cache: SharedCache,
}

#[tonic::async_trait]
impl CacheService for MyCacheService {
    async fn internal_set(
        &self,
        request: Request<SetRequest>,
    ) -> Result<Response<SetResponse>, Status> {
        let req = request.into_inner();
        let key = req.key;
        let value_json = req.value_json;
        let value: Value = match serde_json::from_str(&value_json) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse JSON value for key {}: {}", key, e);
                return Err(Status::invalid_argument("Invalid JSON value provided"));
            }
        };

        let ttl = match req.ttl_option {
            Some(proto_cache::set_request::TtlOption::UseDefaultTtl(true)) => CacheItemTTL::Default,
            Some(proto_cache::set_request::TtlOption::SetPermanent(true)) => {
                CacheItemTTL::Permanent
            }
            Some(proto_cache::set_request::TtlOption::SpecificTtlSeconds(sec)) => {
                CacheItemTTL::Custom(Duration::from_secs(sec))
            }
            _ => CacheItemTTL::Default,
        };

        self.cache.set(key, value, ttl).await;

        Ok(Response::new(SetResponse {}))
    }

    async fn internal_get(
        &self,
        request: Request<GetRequest>,
    ) -> Result<Response<GetResponse>, Status> {
        let req = request.into_inner();
        let key = req.key;

        match self.cache.get(&key).await {
            Some(value) => {
                let value_json =
                    serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
                Ok(Response::new(GetResponse { value_json }))
            }
            None => {
                Err(Status::not_found(format!("Key '{}' not found", key)))
            }
        }
    }

    async fn internal_delete(
        &self,
        request: Request<DeleteRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let req = request.into_inner();
        let key = req.key;

        let deleted_count = self.cache.delete(&key).await;

        Ok(Response::new(DeleteResponse { deleted_count }))
    }
}

pub async fn run_rpc_server(
    settings: SharedSettings,
    cache: SharedCache,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = settings.rpc_addr.parse()?;

    let service = MyCacheService { cache };

    info!("gRPC server (Internal) listening on {}", addr);

    Server::builder()
        .add_service(CacheServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
