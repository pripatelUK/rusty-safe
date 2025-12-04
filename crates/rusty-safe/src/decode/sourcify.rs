//! Sourcify 4byte signature lookup with caching
//!
//! Uses Sourcify's Signature Database API:
//! https://docs.sourcify.dev/docs/api/#/Signature%20Database/get_signature_database_v1_lookup
//!
//! Cache is persisted to LocalStorage on WASM for cross-session persistence.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

const SOURCIFY_API: &str = "https://api.4byte.sourcify.dev/signature-database/v1/lookup";

/// How many requests can fail before we mark the connection as spurious
const MAX_FAILED_REQUESTS: usize = 3;

/// LocalStorage key for signature cache
const STORAGE_KEY: &str = "rusty-safe-signatures-v1";

/// Maximum cached selectors (to prevent unbounded storage growth)
const MAX_CACHED_SELECTORS: usize = 1000;

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

/// Cached 4byte signature lookup client with spurious connection detection
///
/// Tracks failed requests and marks the API as unavailable after
/// `MAX_FAILED_REQUESTS` consecutive failures (timeout, network error, 5xx).
#[derive(Clone)]
pub struct SignatureLookup {
    cache: Arc<Mutex<HashMap<String, Vec<String>>>>,
    /// Whether the API connection appears to be down
    is_spurious: Arc<AtomicBool>,
    /// Count of consecutive failed requests
    failed_count: Arc<AtomicUsize>,
}

impl Default for SignatureLookup {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable cache format for LocalStorage
#[derive(Serialize, Deserialize, Default)]
struct StoredCache {
    signatures: HashMap<String, Vec<String>>,
}

impl SignatureLookup {
    pub fn new() -> Self {
        // Load cached signatures from LocalStorage (WASM only)
        let cached = Self::load_from_storage();
        debug_log!("Loaded {} cached signatures from storage", cached.len());
        
        Self {
            cache: Arc::new(Mutex::new(cached)),
            is_spurious: Arc::new(AtomicBool::new(false)),
            failed_count: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    /// Load cache from LocalStorage (WASM only)
    #[cfg(target_arch = "wasm32")]
    fn load_from_storage() -> HashMap<String, Vec<String>> {
        use gloo_storage::{LocalStorage, Storage};
        
        LocalStorage::get::<StoredCache>(STORAGE_KEY)
            .map(|c| c.signatures)
            .unwrap_or_default()
    }
    
    /// Load cache - no-op on native
    #[cfg(not(target_arch = "wasm32"))]
    fn load_from_storage() -> HashMap<String, Vec<String>> {
        HashMap::new()
    }
    
    /// Save cache to LocalStorage (WASM only)
    #[cfg(target_arch = "wasm32")]
    fn save_to_storage(&self) {
        use gloo_storage::{LocalStorage, Storage};
        
        let cache = self.cache.lock().unwrap();
        
        // Limit cache size to prevent unbounded growth
        let signatures: HashMap<String, Vec<String>> = if cache.len() > MAX_CACHED_SELECTORS {
            // Keep only the most recently added (take last N)
            cache.iter()
                .take(MAX_CACHED_SELECTORS)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            cache.clone()
        };
        
        let stored = StoredCache { signatures };
        if let Err(e) = LocalStorage::set(STORAGE_KEY, &stored) {
            debug_log!("Failed to save cache to LocalStorage: {}", e);
        } else {
            debug_log!("Saved {} signatures to LocalStorage", stored.signatures.len());
        }
    }
    
    /// Save cache - no-op on native
    #[cfg(not(target_arch = "wasm32"))]
    fn save_to_storage(&self) {
        // No-op on native
    }

    /// Check if the API appears to be down
    pub fn is_spurious(&self) -> bool {
        self.is_spurious.load(Ordering::Relaxed)
    }

    /// Reset spurious state (e.g., for retry)
    pub fn reset_spurious(&self) {
        self.is_spurious.store(false, Ordering::Relaxed);
        self.failed_count.store(0, Ordering::Relaxed);
    }

    /// Record a successful request
    fn on_success(&self) {
        self.failed_count.store(0, Ordering::Relaxed);
    }

    /// Record a failed request
    fn on_failure(&self, err: &reqwest::Error) {
        // Only count connectivity-type errors (timeout, or server errors)
        // Note: is_connect() isn't available in WASM, so we check timeout and server errors
        let is_connectivity_error = err.is_timeout() 
            || err.status().map_or(false, |s| s.is_server_error())
            || err.is_request();  // Catch other request-level failures
        
        if is_connectivity_error {
            let count = self.failed_count.fetch_add(1, Ordering::SeqCst) + 1;
            debug_log!("Request failed ({}/{}): {}", count, MAX_FAILED_REQUESTS, err);
            if count >= MAX_FAILED_REQUESTS {
                debug_log!("Marking API as spurious after {} failures", count);
                self.is_spurious.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Lookup a single selector (checks cache first)
    pub async fn lookup(&self, selector: &str) -> Result<Vec<String>> {
        // Short-circuit if API is marked as down
        if self.is_spurious() {
            debug_log!("Skipping lookup - API marked as spurious");
            return Ok(vec![]);
        }

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

        // Cache result and persist to storage
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(selector, sigs.clone());
        }
        self.save_to_storage();

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
                {
                    let mut cache = self.cache.lock().unwrap();
                    for (sel, sigs) in fetched {
                        cache.insert(sel.clone(), sigs.clone());
                        results.insert(sel, sigs);
                    }
                }
                // Persist to storage after batch update
                self.save_to_storage();
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
    async fn fetch_single(&self, selector: &str) -> Result<Vec<String>> {
        self.fetch_batch(&[selector.to_string()])
            .await
            .map(|mut map| map.remove(selector).unwrap_or_default())
    }

    /// Fetch signatures for multiple selectors in one request
    async fn fetch_batch(&self, selectors: &[String]) -> Result<HashMap<String, Vec<String>>> {
        if selectors.is_empty() {
            return Ok(HashMap::new());
        }

        // Short-circuit if API is marked as down
        if self.is_spurious() {
            debug_log!("Skipping batch fetch - API marked as spurious");
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

        let response = match reqwest::get(&url).await {
            Ok(resp) => resp,
            Err(e) => {
                self.on_failure(&e);
                return Err(e).wrap_err("Failed to fetch from Sourcify API");
            }
        };

        debug_log!("Response status: {}", response.status());

        if !response.status().is_success() {
            // Track server errors as potential spurious connection
            if response.status().is_server_error() {
                let count = self.failed_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= MAX_FAILED_REQUESTS {
                    self.is_spurious.store(true, Ordering::Relaxed);
                }
            }
            eyre::bail!("Sourcify API error: {}", response.status());
        }

        let api_response: SourcifyResponse = response
            .json()
            .await
            .wrap_err("Failed to parse Sourcify response")?;

        if !api_response.ok {
            eyre::bail!("Sourcify API returned ok=false");
        }

        // Success - reset failure count
        self.on_success();

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


