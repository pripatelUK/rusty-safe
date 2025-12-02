//! Sourcify 4byte signature lookup with caching
//!
//! Uses Sourcify's Signature Database API:
//! https://docs.sourcify.dev/docs/api/#/Signature%20Database/get_signature_database_v1_lookup

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::Deserialize;

const SOURCIFY_API: &str = "https://api.4byte.sourcify.dev/signature-database/v1/lookup";

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

/// Response from Sourcify Signature Database API
#[derive(Debug, Deserialize)]
struct SourcifyResponse {
    ok: bool,
    result: SourcifyResult,
}

#[derive(Debug, Deserialize)]
struct SourcifyResult {
    function: HashMap<String, Vec<SignatureEntry>>,
    #[allow(dead_code)]
    event: HashMap<String, Vec<SignatureEntry>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignatureEntry {
    name: String,
    #[allow(dead_code)]
    filtered: bool,
    #[allow(dead_code)]
    has_verified_contract: Option<bool>,
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
                    debug_log!("Cache hit for {}", normalized);
                    results.insert(normalized, sigs.clone());
                } else if !to_fetch.contains(&normalized) {
                    to_fetch.push(normalized);
                }
            }
        }

        if to_fetch.is_empty() {
            return results;
        }

        debug_log!("Batch fetching {} selectors: {:?}", to_fetch.len(), to_fetch);

        // Fetch all uncached in one request
        match self.fetch_batch(&to_fetch).await {
            Ok(fetched) => {
                let mut cache = self.cache.lock().unwrap();
                for (sel, sigs) in fetched {
                    cache.insert(sel.clone(), sigs.clone());
                    results.insert(sel, sigs);
                }
            }
            Err(e) => {
                debug_log!("Batch fetch error: {}", e);
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

    /// Fetch signatures for a selector from Sourcify
    async fn fetch_single(&self, selector: &str) -> Result<Vec<String>, String> {
        self.fetch_batch(&[selector.to_string()])
            .await
            .map(|mut map| map.remove(selector).unwrap_or_default())
    }

    /// Fetch signatures for multiple selectors in one request
    async fn fetch_batch(&self, selectors: &[String]) -> Result<HashMap<String, Vec<String>>, String> {
        if selectors.is_empty() {
            return Ok(HashMap::new());
        }

        // Build URL with function params (can have multiple)
        // e.g., ?function=0xa9059cbb&function=0x095ea7b3&filter=true
        let params: Vec<String> = selectors
            .iter()
            .map(|s| format!("function={}", s))
            .collect();
        let url = format!("{}?{}&filter=true", SOURCIFY_API, params.join("&"));
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

        let api_response: SourcifyResponse = response
            .json()
            .await
            .map_err(|e| {
                debug_log!("Parse error: {}", e);
                format!("Parse error: {}", e)
            })?;

        if !api_response.ok {
            return Err("API returned ok=false".into());
        }

        // Convert to our format, prioritizing verified signatures
        let mut results = HashMap::new();
        for (selector, entries) in api_response.result.function {
            // Sort: verified contracts first
            let mut sigs: Vec<(bool, String)> = entries
                .into_iter()
                .map(|e| (e.has_verified_contract.unwrap_or(false), e.name))
                .collect();
            sigs.sort_by(|a, b| b.0.cmp(&a.0)); // verified first

            let sig_names: Vec<String> = sigs.into_iter().map(|(_, name)| name).collect();
            debug_log!("Selector {}: {:?}", selector, sig_names);
            results.insert(selector, sig_names);
        }

        Ok(results)
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


