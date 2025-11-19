// src/http_server.rs
use crate::{
    cache::{CacheItemTTL, SharedCache},
    cluster::SharedCluster,
    config::SharedSettings,
    error::AppError,
    rpc_client::RpcClient,
};
#[allow(unused_imports)]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use log::info;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    cache: SharedCache,
    cluster: SharedCluster,
    rpc_client: RpcClient,
}

pub async fn run_http_server(
    settings: SharedSettings,
    cache: SharedCache,
    cluster: SharedCluster,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = settings.http_addr.parse()?;

    let rpc_client = RpcClient::new(3000); // 3 秒连接超时

    let app_state = AppState {
        cache,
        cluster,
        rpc_client,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", post(handler_post_set)) // [cite: 13]
        .route("/:key", get(handler_get)) // [cite: 15]
        .route("/:key", delete(handler_delete)) // [cite: 19]
        .with_state(app_state) // 注入共享状态
        .layer(cors);

    info!("HTTP server (Client Entry) listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

#[derive(Debug, Deserialize)]
struct PostQuery {
    ttl: Option<String>,
}

async fn handler_post_set(
    State(state): State<AppState>,
    Query(query): Query<PostQuery>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, AppError> {
    let mut map = payload
        .as_object()
        .ok_or_else(|| AppError::InvalidInput("Body must be a JSON object".to_string()))?
        .clone();

    if map.len() != 1 {
        return Err(AppError::InvalidInput(
            "JSON body must contain exactly one key-value pair".to_string(), //
        ));
    }

    let (key, value) = map
        .remove_entry(map.clone().keys().next().unwrap())
        .unwrap();
    info!("Received SET for '{}' - '{}'", key, value);

    let ttl = parse_ttl_query(query.ttl);

    let target_addr = state.cluster.get_node_for_key(&key);

    if target_addr == state.cluster.my_addr {
        info!("Handling SET locally : {} - {}", key, value);
        state.cache.set(key, value, ttl).await;
    } else {
        info!(
            "Forwarding SET for key-value: '{}' - '{}' to {}",
            key, value, target_addr
        );
        state
            .rpc_client
            .forward_set(key, value, ttl, &target_addr)
            .await?;
    }

    Ok(StatusCode::OK)
}

async fn handler_get(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let value = {
        let target_addr = state.cluster.get_node_for_key(&key);

        if target_addr == state.cluster.my_addr {
            info!("Handling GET for key '{}' locally", key);
            state.cache.get(&key).await
        } else {
            info!("Forwarding GET for key '{}' to {}", key, target_addr);
            state.rpc_client.forward_get(&key, &target_addr).await.ok()
        }
    };

    match value {
        Some(val) => {
            let mut result_map = Map::new();
            result_map.insert(key, val);
            Ok((StatusCode::OK, Json(json!(result_map))))
        }
        None => Err(AppError::KeyNotFound),
    }
}

async fn handler_delete(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let deleted_count = {
        let target_addr = state.cluster.get_node_for_key(&key);

        if target_addr == state.cluster.my_addr {
            info!("Handling DELETE for key '{}' locally", key);
            state.cache.delete(&key).await
        } else {
            info!("Forwarding DELETE for key '{}' to {}", key, target_addr);
            state
                .rpc_client
                .forward_delete(&key, &target_addr)
                .await
                .unwrap_or(0)
        }
    };

    Ok((StatusCode::OK, Json(deleted_count)))
}

fn parse_ttl_query(ttl_str: Option<String>) -> CacheItemTTL {
    match ttl_str.as_deref() {
        Some("permanent") => CacheItemTTL::Permanent,
        Some(s) => {
            if let Ok(sec) = s.parse::<u64>() {
                CacheItemTTL::Custom(Duration::from_secs(sec))
            } else {
                CacheItemTTL::Default
            }
        }
        None => CacheItemTTL::Default,
    }
}
