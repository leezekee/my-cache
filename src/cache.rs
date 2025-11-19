// src/cache.rs

use moka::future::Cache;
use serde_json::Value;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub enum CacheItemTTL{
    Default,
    Permanent,
    Custom(Duration),
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    value: Value,
    expires_at: Option<Instant>,
}

impl Hash for CacheEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.to_string().hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct CacheStore {
    store: Cache<String, CacheEntry>,
    default_ttl: Duration,
}

pub type SharedCache = Arc<CacheStore>;


impl CacheStore {
    pub fn new(capacity: u64, default_ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(capacity)
            .build();
        // let cache = Cache::new(capacity);
        Self { 
            store: cache,
            default_ttl: Duration::from_secs(default_ttl_seconds)
        }
    }

    pub async fn set(&self, key: String, value: Value, ttl: CacheItemTTL) {
        // self.store.insert(key, value).await;
        let expires_at = match ttl {
            CacheItemTTL::Default => Some(Instant::now() + self.default_ttl),
            CacheItemTTL::Permanent => None,
            CacheItemTTL::Custom(dur) => Some(Instant::now() + dur),
        };
        let entry = CacheEntry { value, expires_at };
        self.store.insert(key, entry).await;
    }

    pub async fn get(&self, key: &str) -> Option<Value> {
        let Some(entry) = self.store.get(key).await else {
            return None;
        };
        if let Some(expiration) = entry.expires_at {
            if Instant::now() >= expiration {
                self.store.remove(key).await;
                return None;
            }
        }
        Some(entry.value.clone())
    }

    pub async fn delete(&self, key: &str) -> i64 {
        match self.store.remove(key).await {
            Some(_) => 1,
            None => 0,  
        }
    }
}


// --- 单元测试 ---
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_cache(capacity: u64, default_ttl_sec: u64) -> CacheStore {
        CacheStore::new(capacity, default_ttl_sec)
    }
    
    #[tokio::test]
    async fn test_lru_eviction_still_works() {
        let cache = create_test_cache(2, 60); // 容量为 2

        // 插入 3 个永久条目
        cache.set("key1".to_string(), json!(1), CacheItemTTL::Permanent).await;
        cache.store.run_pending_tasks().await;
        cache.set("key2".to_string(), json!(2), CacheItemTTL::Permanent).await;
        cache.store.run_pending_tasks().await;
        cache.set("key3".to_string(), json!(3), CacheItemTTL::Permanent).await; 
        cache.store.run_pending_tasks().await;
        cache.set("key4".to_string(), json!(4), CacheItemTTL::Permanent).await;
        cache.store.run_pending_tasks().await;
        cache.set("key5".to_string(), json!(5), CacheItemTTL::Permanent).await;

        cache.store.run_pending_tasks().await;

        let count = cache.store.entry_count();
        println!("当前缓存条目数: {}", count);

        // cache.store.run_pending_tasks().await;

        assert_eq!(cache.get("key1").await, None, "key1 应该被 LRU 驱逐");
        assert_eq!(cache.get("key2").await, Some(json!(2)));
        assert_eq!(cache.get("key3").await, Some(json!(3)));
    }
}