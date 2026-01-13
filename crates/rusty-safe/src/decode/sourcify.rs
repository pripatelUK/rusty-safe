//! Sourcify 4byte signature lookup with caching
//!
//! Uses Sourcify's Signature Database API:
//! https://docs.sourcify.dev/docs/api/#/Signature%20Database/get_signature_database_v1_lookup
//!
//! Cache is persisted via eframe storage (works on both WASM and native).

use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const SOURCIFY_API: &str = "https://api.4byte.sourcify.dev/signature-database/v1/lookup";

/// How many requests can fail before we mark the connection as spurious
const MAX_FAILED_REQUESTS: usize = 3;

/// Storage key for signature cache
const SIGNATURES_STORAGE_KEY: &str = "signatures_cache";

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

/// Acquire mutex lock, recovering from poisoned state if necessary.
macro_rules! lock_or_recover {
    ($mutex:expr) => {
        match $mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                debug_log!("Warning: Mutex was poisoned, recovering");
                poisoned.into_inner()
            }
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
    has_verified_contract: Option<bool>,
}

/// A signature with its verification status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureInfo {
    pub signature: String,
    pub verified: bool,
}

/// Cached 4byte signature lookup client with spurious connection detection
///
/// Tracks failed requests and marks the API as unavailable after
/// `MAX_FAILED_REQUESTS` consecutive failures (timeout, network error, 5xx).
#[derive(Clone)]
pub struct SignatureLookup {
    cache: Arc<Mutex<HashMap<String, Vec<SignatureInfo>>>>,
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

/// Serializable cache format for storage
#[derive(Serialize, Deserialize, Default)]
struct StoredCache {
    signatures: HashMap<String, Vec<SignatureInfo>>,
}

impl SignatureLookup {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            is_spurious: Arc::new(AtomicBool::new(false)),
            failed_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Load cache from eframe storage
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        let cache = if let Some(storage) = storage {
            storage
                .get_string(SIGNATURES_STORAGE_KEY)
                .and_then(|s| serde_json::from_str::<StoredCache>(&s).ok())
                .map(|c| c.signatures)
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        debug_log!("Loaded {} cached signatures from storage", cache.len());

        Self {
            cache: Arc::new(Mutex::new(cache)),
            is_spurious: Arc::new(AtomicBool::new(false)),
            failed_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Save cache to eframe storage
    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        let cache = lock_or_recover!(self.cache);

        // Limit cache size to prevent unbounded growth
        let signatures: HashMap<String, Vec<SignatureInfo>> = if cache.len() > MAX_CACHED_SELECTORS
        {
            cache
                .iter()
                .take(MAX_CACHED_SELECTORS)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            cache.clone()
        };

        let stored = StoredCache { signatures };
        if let Ok(json) = serde_json::to_string(&stored) {
            storage.set_string(SIGNATURES_STORAGE_KEY, json);
            debug_log!("Saved {} signatures to storage", stored.signatures.len());
        }
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
            || err.is_request(); // Catch other request-level failures

        if is_connectivity_error {
            let count = self.failed_count.fetch_add(1, Ordering::SeqCst) + 1;
            debug_log!(
                "Request failed ({}/{}): {}",
                count,
                MAX_FAILED_REQUESTS,
                err
            );
            if count >= MAX_FAILED_REQUESTS {
                debug_log!("Marking API as spurious after {} failures", count);
                self.is_spurious.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Lookup a single selector (checks cache first)
    /// Returns signatures with verification status, sorted by verified first
    pub async fn lookup(&self, selector: &str) -> Result<Vec<SignatureInfo>> {
        // Short-circuit if API is marked as down
        if self.is_spurious() {
            debug_log!("Skipping lookup - API marked as spurious");
            return Ok(vec![]);
        }

        let selector = normalize_selector(selector);
        debug_log!("Looking up selector: {}", selector);

        // Check cache
        {
            let cache = lock_or_recover!(self.cache);
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
            let mut cache = lock_or_recover!(self.cache);
            cache.insert(selector, sigs.clone());
        }

        Ok(sigs)
    }

    /// Batch lookup multiple selectors (deduplicates, uses cache)
    /// Returns signatures with verification status for each selector
    pub async fn lookup_batch(&self, selectors: &[String]) -> HashMap<String, Vec<SignatureInfo>> {
        let mut results = HashMap::new();
        let mut to_fetch = Vec::new();

        // Check cache, collect uncached
        {
            let cache = lock_or_recover!(self.cache);
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

        debug_log!(
            "Batch fetching {} selectors: {:?}",
            to_fetch.len(),
            to_fetch
        );

        // Fetch all uncached in one request
        match self.fetch_batch(&to_fetch).await {
            Ok(fetched) => {
                let mut cache = lock_or_recover!(self.cache);
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
        let cache = lock_or_recover!(self.cache);
        cache.contains_key(&selector)
    }

    /// Fetch signatures for a selector from Sourcify
    async fn fetch_single(&self, selector: &str) -> Result<Vec<SignatureInfo>> {
        self.fetch_batch(&[selector.to_string()])
            .await
            .map(|mut map| map.remove(selector).unwrap_or_default())
    }

    /// Fetch signatures for multiple selectors in one request
    async fn fetch_batch(
        &self,
        selectors: &[String],
    ) -> Result<HashMap<String, Vec<SignatureInfo>>> {
        if selectors.is_empty() {
            return Ok(HashMap::new());
        }

        // Short-circuit if API is marked as down
        if self.is_spurious() {
            debug_log!("Skipping batch fetch - API marked as spurious");
            return Ok(HashMap::new());
        }

        // Build URL with comma-delimited selectors
        // e.g., ?function=0xa9059cbb,0x095ea7b3&filter=true
        let selectors_csv: String = selectors
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let url = format!("{}?function={}&filter=true", SOURCIFY_API, selectors_csv);
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

        // Convert to our format, preserving verification status
        // Sort: verified contracts first
        let mut results = HashMap::new();
        for (selector, entries) in api_response.result.function {
            let mut sigs: Vec<SignatureInfo> = entries
                .into_iter()
                .map(|e| SignatureInfo {
                    signature: e.name,
                    verified: e.has_verified_contract.unwrap_or(false),
                })
                .collect();
            // Sort: verified first
            sigs.sort_by(|a, b| b.verified.cmp(&a.verified));

            debug_log!("Selector {}: {:?}", selector, sigs);
            results.insert(selector, sigs);
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
