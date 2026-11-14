// SPDX-License-Identifier: MIT OR Apache-2.0

use parking_lot::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use url::Url;

/// Async DNS prefetcher triggered on omnibox hover/typing to cut navigation latency.
pub struct DnsPrefetcher {
    recent_hosts: Mutex<HashMap<String, Instant>>,
    ttl: Duration,
}

impl Default for DnsPrefetcher {
    fn default() -> Self {
        Self::new(Duration::from_secs(120))
    }
}

impl DnsPrefetcher {
    pub fn new(ttl: Duration) -> Self {
        Self {
            recent_hosts: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    pub fn prefetch_url(&self, raw_url: &str) {
        let Ok(parsed) = Url::parse(raw_url) else {
            return;
        };
        let Some(host) = parsed.host_str().map(str::to_string) else {
            return;
        };
        let port = parsed.port_or_known_default().unwrap_or(443);

        {
            let mut guard = self.recent_hosts.lock();
            if let Some(prev) = guard.get(&host)
                && prev.elapsed() < self.ttl
            {
                return;
            }
            guard.insert(host.clone(), Instant::now());
        }

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = tokio::net::lookup_host((host.as_str(), port)).await;
            });
        }
    }
}
