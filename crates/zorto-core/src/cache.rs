use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Directory name for the cache inside the site root.
const CACHE_DIR: &str = ".zorto/cache";

/// A single cached block result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedBlock {
    /// SHA-256 hex digest of the block source code.
    pub hash: String,
    /// Captured stdout (may be empty).
    pub output: Option<String>,
    /// Captured stderr / error message (may be empty).
    pub error: Option<String>,
}

/// In-memory representation of a page's cached blocks.
///
/// Stored on disk as `.zorto/cache/{page_key_hash}.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageCache {
    /// Cached blocks indexed by their position (0-based) within the page.
    pub blocks: HashMap<String, CachedBlock>,
}

/// Compute the SHA-256 hex digest of a string.
pub fn hash_source(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Return the cache directory path for a site root.
pub fn cache_dir(site_root: &Path) -> PathBuf {
    site_root.join(CACHE_DIR)
}

/// Derive a filesystem-safe name for a page key.
fn page_cache_file(site_root: &Path, page_key: &str) -> PathBuf {
    // Hash the page key to avoid filesystem issues with slashes/special chars.
    let mut hasher = Sha256::new();
    hasher.update(page_key.as_bytes());
    let name = format!("{:x}", hasher.finalize());
    cache_dir(site_root).join(format!("{name}.json"))
}

/// Load the cached blocks for a page, if the cache file exists.
pub fn load_page_cache(site_root: &Path, page_key: &str) -> Option<PageCache> {
    let path = page_cache_file(site_root, page_key);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Save the cached blocks for a page.
pub fn save_page_cache(site_root: &Path, page_key: &str, cache: &PageCache) -> anyhow::Result<()> {
    let path = page_cache_file(site_root, page_key);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cache)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Remove the entire cache directory.
pub fn clear_cache(site_root: &Path) -> anyhow::Result<()> {
    let dir = cache_dir(site_root);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hash_source_deterministic() {
        let h1 = hash_source("print('hello')");
        let h2 = hash_source("print('hello')");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_source_different_inputs() {
        let h1 = hash_source("print('hello')");
        let h2 = hash_source("print('world')");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_save_and_load_page_cache() {
        let tmp = TempDir::new().unwrap();
        let page_key = "blog/post.md";

        let mut cache = PageCache::default();
        cache.blocks.insert(
            "0".to_string(),
            CachedBlock {
                hash: hash_source("echo hello"),
                output: Some("hello\n".to_string()),
                error: None,
            },
        );

        save_page_cache(tmp.path(), page_key, &cache).unwrap();
        let loaded = load_page_cache(tmp.path(), page_key).unwrap();
        assert_eq!(loaded.blocks.len(), 1);
        let block = &loaded.blocks["0"];
        assert_eq!(block.output.as_deref(), Some("hello\n"));
        assert!(block.error.is_none());
    }

    #[test]
    fn test_load_missing_cache_returns_none() {
        let tmp = TempDir::new().unwrap();
        assert!(load_page_cache(tmp.path(), "nonexistent.md").is_none());
    }

    #[test]
    fn test_clear_cache() {
        let tmp = TempDir::new().unwrap();
        let cache = PageCache::default();
        save_page_cache(tmp.path(), "test.md", &cache).unwrap();
        assert!(cache_dir(tmp.path()).exists());

        clear_cache(tmp.path()).unwrap();
        assert!(!cache_dir(tmp.path()).exists());
    }

    #[test]
    fn test_clear_cache_noop_when_missing() {
        let tmp = TempDir::new().unwrap();
        // Should not error when cache dir doesn't exist
        clear_cache(tmp.path()).unwrap();
    }

    #[test]
    fn test_corrupted_json_returns_none() {
        let tmp = TempDir::new().unwrap();
        let page_key = "test.md";
        // Write a valid cache first to get the file path, then corrupt it
        let cache = PageCache::default();
        save_page_cache(tmp.path(), page_key, &cache).unwrap();
        let cache_file = page_cache_file(tmp.path(), page_key);
        std::fs::write(&cache_file, "not valid json {{{").unwrap();
        // Should gracefully return None instead of panicking
        assert!(load_page_cache(tmp.path(), page_key).is_none());
    }

    #[test]
    fn test_cache_hit_and_miss() {
        let tmp = TempDir::new().unwrap();
        let page_key = "test.md";
        let source = "echo cached";
        let source_hash = hash_source(source);

        // Save a cached result
        let mut cache = PageCache::default();
        cache.blocks.insert(
            "0".to_string(),
            CachedBlock {
                hash: source_hash.clone(),
                output: Some("cached\n".to_string()),
                error: None,
            },
        );
        save_page_cache(tmp.path(), page_key, &cache).unwrap();

        // Load and check hit
        let loaded = load_page_cache(tmp.path(), page_key).unwrap();
        let block = &loaded.blocks["0"];
        assert_eq!(block.hash, source_hash);
        assert_eq!(block.output.as_deref(), Some("cached\n"));

        // Different source => miss
        let new_hash = hash_source("echo different");
        assert_ne!(block.hash, new_hash);
    }
}
