//! Sourcify 4byte signature lookup with caching

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::Deserialize;

const SOURCIFY_API: &str = "https://www.4byte.directory/api/v1/signatures";

/// Log to console (works in both WASM and native)
macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&format!($($arg)*).into());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            eprintln!("[4byte] {}", format!($($arg)*));
        }
    };
}

/// Response from 4byte.directory API
#[derive(Debug, Deserialize)]
struct FourByteResponse {
    count: u32,
    results: Vec<FourByteResult>,
}

#[derive(Debug, Deserialize)]
struct FourByteResult {
    text_signature: String,
}

/// Cached 4byte signature lookup client
#[derive(Clone)]
pub struct SignatureLookup {
    cache: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl Default for SignatureLookup {
    fn default() -> Self {
        Self::new()
    }
}

impl SignatureLookup {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Lookup a single selector (checks cache first)
    pub async fn lookup(&self, selector: &str) -> Result<Vec<String>, String> {
        let selector = normalize_selector(selector);
        debug_log!("Looking up selector: {}", selector);

        // Check cache
        {
            let cache = self.cache.lock().unwrap();
            if let Some(sigs) = cache.get(&selector) {
                debug_log!("Cache hit for {}: {} signatures", selector, sigs.len());
                return Ok(sigs.clone());
            }
        }
        debug_log!("Cache miss for {}, fetching from API...", selector);

        // Fetch from API
        let sigs = self.fetch_single(&selector).await?;
        debug_log!("Fetched {} signatures for {}", sigs.len(), selector);

        // Cache result
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(selector, sigs.clone());
        }

        Ok(sigs)
    }

    /// Batch lookup multiple selectors (deduplicates, uses cache)
    pub async fn lookup_batch(&self, selectors: &[String]) -> HashMap<String, Vec<String>> {
        let mut results = HashMap::new();
        let mut to_fetch = Vec::new();

        // Check cache, collect uncached
        {
            let cache = self.cache.lock().unwrap();
            for sel in selectors {
                let normalized = normalize_selector(sel);
                if let Some(sigs) = cache.get(&normalized) {
                    results.insert(normalized, sigs.clone());
                } else if !to_fetch.contains(&normalized) {
                    to_fetch.push(normalized);
                }
            }
        }

        // Fetch uncached (one by one for now, could parallelize)
        for sel in to_fetch {
            if let Ok(sigs) = self.fetch_single(&sel).await {
                let mut cache = self.cache.lock().unwrap();
                cache.insert(sel.clone(), sigs.clone());
                results.insert(sel, sigs);
            }
        }

        results
    }

    /// Check if selector is cached
    pub fn is_cached(&self, selector: &str) -> bool {
        let selector = normalize_selector(selector);
        let cache = self.cache.lock().unwrap();
        cache.contains_key(&selector)
    }

    /// Fetch signatures for a selector from 4byte.directory
    async fn fetch_single(&self, selector: &str) -> Result<Vec<String>, String> {
        // 4byte.directory uses hex_signature param without 0x prefix
        let hex_sig = selector.strip_prefix("0x").unwrap_or(selector);
        let url = format!("{}/?hex_signature={}", SOURCIFY_API, hex_sig);
        debug_log!("Fetching: {}", url);

        let response = reqwest::get(&url)
            .await
            .map_err(|e| {
                debug_log!("Network error: {}", e);
                format!("Network error: {}", e)
            })?;

        debug_log!("Response status: {}", response.status());

        if !response.status().is_success() {
            return Err(format!("API error: {}", response.status()));
        }

        let api_response: FourByteResponse = response
            .json()
            .await
            .map_err(|e| {
                debug_log!("Parse error: {}", e);
                format!("Parse error: {}", e)
            })?;

        let sigs: Vec<String> = api_response
            .results
            .into_iter()
            .map(|r| r.text_signature)
            .collect();

        debug_log!("Found signatures: {:?}", sigs);
        Ok(sigs)
    }
}

/// Normalize selector to lowercase with 0x prefix
fn normalize_selector(selector: &str) -> String {
    let sel = selector.trim().to_lowercase();
    if sel.starts_with("0x") {
        sel
    } else {
        format!("0x{}", sel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_selector() {
        assert_eq!(normalize_selector("0xA9059CBB"), "0xa9059cbb");
        assert_eq!(normalize_selector("a9059cbb"), "0xa9059cbb");
        assert_eq!(normalize_selector("0xa9059cbb"), "0xa9059cbb");
    }
}


