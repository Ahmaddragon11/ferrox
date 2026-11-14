// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::cache::MemoryAssetCache;
use ferrox_core::{FerroxError, Result};
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct FetchedResource {
    pub url: String,
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub elapsed_ms: u64,
    pub from_cache: bool,
}

#[derive(Clone)]
pub struct FastNetClient {
    client: Client,
    cache: Arc<MemoryAssetCache>,
}

impl FastNetClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Ferrox/0.1.0")
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(25))
            .pool_max_idle_per_host(8)
            .build()
            .map_err(|e| FerroxError::Network(e.to_string()))?;

        Ok(Self {
            client,
            cache: Arc::new(MemoryAssetCache::new(32 * 1024 * 1024)),
        })
    }

    pub fn raw_client(&self) -> &Client {
        &self.client
    }

    pub fn cache(&self) -> &MemoryAssetCache {
        &self.cache
    }

    pub async fn fetch_cached(&self, url: &str, use_cache: bool) -> Result<FetchedResource> {
        if use_cache && let Some(hit) = self.cache.get(url) {
            return Ok(FetchedResource {
                url: url.to_string(),
                status: 200,
                content_type: hit.content_type,
                body: hit.bytes,
                elapsed_ms: 0,
                from_cache: true,
            });
        }

        let start = Instant::now();
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FerroxError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let final_url = resp.url().to_string();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let body = resp
            .bytes()
            .await
            .map_err(|e| FerroxError::Network(e.to_string()))?
            .to_vec();

        let elapsed_ms = start.elapsed().as_millis() as u64;

        if use_cache && status == 200 && body.len() <= 2 * 1024 * 1024 {
            self.cache.insert(
                url,
                body.clone(),
                content_type.clone(),
                Duration::from_secs(120),
            );
        }

        Ok(FetchedResource {
            url: final_url,
            status,
            content_type,
            body,
            elapsed_ms,
            from_cache: false,
        })
    }
}
