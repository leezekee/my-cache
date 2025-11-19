// src/cluster.rs
#[allow(unused_imports)]
use crate::config::{Settings, SharedSettings};
use mpchash::HashRing;
use std::sync::Arc;

#[derive(Clone)]
pub struct Cluster {
    ring: HashRing<String>,
    
    pub my_addr: String,
}

pub type SharedCluster = Arc<Cluster>;

impl Cluster {
    pub fn new(settings: &Settings) -> Self {
        let ring = HashRing::new(); // <-- API 更改

        for node_addr in &settings.cluster_nodes {
            ring.add(node_addr.clone()); // <-- API 更改
        }

        Self {
            ring,
            my_addr: settings.my_connectable_addr.clone(),
        }
    }

    pub fn get_node_for_key(&self, key: &str) -> String {
        self.ring
            .node(&String::from(key))
            .unwrap()
            .to_string()
            .clone()
    }

    #[allow(dead_code)]
    pub fn is_key_local(&self, key: &str) -> bool {
        let target_addr = self.get_node_for_key(key);
        target_addr == self.my_addr
    }
}
