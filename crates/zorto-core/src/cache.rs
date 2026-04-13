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
    /// Captured visualizations as (kind, data) tuples.
    #[serde(default)]
    pub viz: Vec<(String, String)>,
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

/// Compute a cache key for an executable block.
///
/// For inline blocks (no `file_ref`) this matches the legacy
/// `hash_source("{language}:{source}")` format so existing on-disk caches
/// remain valid.
///
/// For `file_ref` blocks, the key includes the language, the inline source,
/// the referenced path, and the file's contents — so editing the referenced
/// script invalidates the cache, and two `file=` blocks pointing at different
/// files no longer collide on an empty inline body.
///
/// `working_dir` is the directory the executor would resolve `file_ref` against
/// (typically the page's content directory). If the file is unreadable, a stable
/// `<unreadable>` marker is hashed in its place — re-runs after the file appears
/// will naturally miss and re-execute.
pub fn block_cache_key(
    language: &str,
    source: &str,
    file_ref: Option<&str>,
    working_dir: &Path,
) -> String {
    let Some(file) = file_ref else {
        return hash_source(&format!("{language}:{source}"));
    };
    let mut hasher = Sha256::new();
    hasher.update(b"v1\x00");
    hasher.update(language.as_bytes());
    hasher.update(b"\x00");
    hasher.update(source.as_bytes());
    hasher.update(b"\x00file=");
    hasher.update(file.as_bytes());
    hasher.update(b"\x00");
    match std::fs::read(working_dir.join(file)) {
        Ok(bytes) => hasher.update(&bytes),
        Err(_) => hasher.update(b"<unreadable>"),
    }
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
                viz: Vec::new(),
            },
        );

        save_page_cache(tmp.path(), page_key, &cache).unwrap();
        let loaded = load_page_cache(tmp.path(), page_key).unwrap();
        assert_eq!(loaded.blocks.len(), 1);
        let block = &loaded.blocks["0"];
        assert_eq!(block.output.as_deref(), Some("hello\n"));
        assert!(block.error.is_none());
        assert!(block.viz.is_empty());
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
                viz: Vec::new(),
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

    #[test]
    fn test_hash_source_deterministic_across_calls() {
        // Verify hash stability across many repeated calls.
        let source = "fn main() { println!(\"hello\"); }";
        let reference = hash_source(source);
        for _ in 0..100 {
            assert_eq!(hash_source(source), reference);
        }
    }

    #[test]
    fn test_block_cache_key_inline_matches_legacy() {
        // Backward-compat: blocks without file_ref must hash to the same value
        // as the previous `hash_source("{language}:{source}")` formulation so
        // existing on-disk caches continue to hit.
        let tmp = TempDir::new().unwrap();
        let key = block_cache_key("python", "print('hi')", None, tmp.path());
        let legacy = hash_source("python:print('hi')");
        assert_eq!(key, legacy);
    }

    #[test]
    fn test_block_cache_key_file_ref_includes_contents() {
        // Regression: two file_ref blocks with empty inline source but different
        // referenced files used to collide on the same hash. Now they're
        // distinct because the file path AND contents are mixed in.
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.py"), "print('a')").unwrap();
        std::fs::write(tmp.path().join("b.py"), "print('b')").unwrap();
        let key_a = block_cache_key("python", "", Some("a.py"), tmp.path());
        let key_b = block_cache_key("python", "", Some("b.py"), tmp.path());
        assert_ne!(
            key_a, key_b,
            "two file_ref blocks pointing at different files must not collide"
        );
    }

    #[test]
    fn test_block_cache_key_busts_on_file_change() {
        // Regression: modifying the referenced file used to leave the cache key
        // unchanged (since the inline source — typically empty — was the only
        // input to the hash).
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("script.py");
        std::fs::write(&path, "print('v1')").unwrap();
        let key_v1 = block_cache_key("python", "", Some("script.py"), tmp.path());
        std::fs::write(&path, "print('v2')").unwrap();
        let key_v2 = block_cache_key("python", "", Some("script.py"), tmp.path());
        assert_ne!(
            key_v1, key_v2,
            "editing the referenced file must invalidate the cache key"
        );
    }

    #[test]
    fn test_block_cache_key_unreadable_file_is_deterministic() {
        // If the referenced file is missing, key generation must not panic and
        // must produce a stable (deterministic) value, so subsequent runs hit
        // the same key — and the moment the file appears, the key changes and
        // the cache misses to re-execute.
        let tmp = TempDir::new().unwrap();
        let key1 = block_cache_key("python", "", Some("missing.py"), tmp.path());
        let key2 = block_cache_key("python", "", Some("missing.py"), tmp.path());
        assert_eq!(key1, key2, "missing-file key must be deterministic");
        std::fs::write(tmp.path().join("missing.py"), "print('appeared')").unwrap();
        let key3 = block_cache_key("python", "", Some("missing.py"), tmp.path());
        assert_ne!(
            key1, key3,
            "key must change once the previously-missing file appears"
        );
    }

    #[test]
    fn test_block_cache_key_inline_vs_file_ref_distinct() {
        // A block whose inline source happens to equal the file's contents
        // must NOT collide with a file_ref block — execution semantics differ
        // (working_dir, error reporting), so cached outputs aren't interchangeable.
        let tmp = TempDir::new().unwrap();
        let body = "print('hi')";
        std::fs::write(tmp.path().join("x.py"), body).unwrap();
        let inline = block_cache_key("python", body, None, tmp.path());
        let from_file = block_cache_key("python", "", Some("x.py"), tmp.path());
        assert_ne!(inline, from_file);
    }
}
