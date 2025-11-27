//! Caching mechanism for parsed Command objects with TTL support.
//!
//! This module provides XDG-compliant caching of parsed help text to avoid
//! re-parsing commands that haven't changed. Cache entries have a configurable
//! TTL (time-to-live) after which they are considered stale.

use crate::types::Command;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use ecow::EcoString;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, trace, warn};

/// Default TTL for cache entries (24 hours in seconds)
pub const DEFAULT_TTL_SECS: u64 = 24 * 60 * 60;

/// A cached Command with metadata for TTL validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Unix timestamp when this entry was created
    pub created_at: u64,
    /// Hash of the input content (help text) for validation
    pub content_hash: u64,
    /// The cached Command object
    pub command: Command,
}

impl CacheEntry {
    /// Create a new cache entry with the current timestamp.
    pub fn new(command: Command, content_hash: u64) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            created_at,
            content_hash,
            command,
        }
    }

    /// Check if this cache entry is still valid (not expired).
    pub fn is_valid(&self, ttl_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let age = now.saturating_sub(self.created_at);
        age < ttl_secs
    }

    /// Check if the content hash matches (content hasn't changed).
    pub fn matches_content(&self, content_hash: u64) -> bool {
        self.content_hash == content_hash
    }
}

/// Cache manager for parsed Command objects.
#[derive(Debug)]
pub struct Cache {
    /// Base directory for cache files
    cache_dir: PathBuf,
    /// TTL in seconds for cache entries
    ttl: Duration,
}

impl Cache {
    /// Create a new Cache instance with the default TTL.
    pub fn new() -> Result<Self> {
        Self::with_ttl(Duration::from_secs(DEFAULT_TTL_SECS))
    }

    /// Create a new Cache instance with a custom TTL.
    pub fn with_ttl(ttl: Duration) -> Result<Self> {
        let cache_dir = Self::get_cache_dir()?;
        Ok(Self { cache_dir, ttl })
    }

    /// Get the XDG-compliant cache directory for hcl.
    fn get_cache_dir() -> Result<PathBuf> {
        let project_dirs =
            ProjectDirs::from("", "", "hcl").context("Failed to determine project directories")?;

        let cache_dir = project_dirs.cache_dir().to_path_buf();
        std::fs::create_dir_all(&cache_dir).with_context(|| {
            format!("Failed to create cache directory: {}", cache_dir.display())
        })?;

        debug!("Using cache directory: {}", cache_dir.display());
        Ok(cache_dir)
    }

    /// Generate a cache key from a command name and optional source identifier.
    fn cache_key(name: &str, source: Option<&str>) -> EcoString {
        let sanitized_name = name.replace(['/', '\\', ':'], "_");
        match source {
            Some(s) => EcoString::from(format!("{}_{:016x}", sanitized_name, Self::hash_string(s))),
            None => EcoString::from(sanitized_name),
        }
    }

    /// Simple FNV-1a hash for string content.
    fn hash_string(s: &str) -> u64 {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        s.bytes().fold(FNV_OFFSET, |hash, byte| {
            (hash ^ byte as u64).wrapping_mul(FNV_PRIME)
        })
    }

    /// Hash content for cache validation.
    pub fn hash_content(content: &str) -> u64 {
        Self::hash_string(content)
    }

    /// Get the path to a cache file for a given key.
    fn cache_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.json", key))
    }

    /// Try to load a cached Command for the given name and source.
    ///
    /// Returns `Some(Command)` if a valid, non-expired cache entry exists
    /// that matches the content hash. Returns `None` otherwise.
    pub async fn get(
        &self,
        name: &str,
        source: Option<&str>,
        content_hash: u64,
    ) -> Option<Command> {
        let key = Self::cache_key(name, source);
        let path = self.cache_path(&key);

        trace!("Looking for cache entry at: {}", path.display());

        let data = match tokio::fs::read_to_string(&path).await {
            Ok(data) => data,
            Err(e) => {
                trace!("Cache miss (read error): {}", e);
                return None;
            }
        };

        let entry: CacheEntry = match serde_json::from_str(&data) {
            Ok(entry) => entry,
            Err(e) => {
                warn!("Cache entry corrupted, removing: {}", e);
                let _ = tokio::fs::remove_file(&path).await;
                return None;
            }
        };

        if !entry.is_valid(self.ttl.as_secs()) {
            debug!("Cache entry expired for: {}", name);
            let _ = tokio::fs::remove_file(&path).await;
            return None;
        }

        if !entry.matches_content(content_hash) {
            debug!("Cache entry content mismatch for: {}", name);
            return None;
        }

        debug!("Cache hit for: {}", name);
        Some(entry.command)
    }

    /// Store a Command in the cache.
    pub async fn set(
        &self,
        name: &str,
        source: Option<&str>,
        content_hash: u64,
        command: &Command,
    ) -> Result<()> {
        let key = Self::cache_key(name, source);
        let path = self.cache_path(&key);

        let entry = CacheEntry::new(command.clone(), content_hash);
        let data =
            serde_json::to_string_pretty(&entry).context("Failed to serialize cache entry")?;

        tokio::fs::write(&path, data)
            .await
            .with_context(|| format!("Failed to write cache entry: {}", path.display()))?;

        debug!("Cached command: {} at {}", name, path.display());
        Ok(())
    }

    /// Clear all cache entries.
    pub async fn clear(&self) -> Result<usize> {
        let mut count = 0;
        let mut entries = tokio::fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                tokio::fs::remove_file(&path).await?;
                count += 1;
            }
        }
        debug!("Cleared {} cache entries", count);
        Ok(count)
    }

    /// Remove expired cache entries.
    pub async fn prune(&self) -> Result<usize> {
        let mut count = 0;
        let mut entries = tokio::fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json")
                && let Ok(data) = tokio::fs::read_to_string(&path).await
                && let Ok(cache_entry) = serde_json::from_str::<CacheEntry>(&data)
                && !cache_entry.is_valid(self.ttl.as_secs())
            {
                tokio::fs::remove_file(&path).await?;
                count += 1;
            }
        }
        debug!("Pruned {} expired cache entries", count);
        Ok(count)
    }

    /// Get cache statistics.
    pub async fn stats(&self) -> Result<CacheStats> {
        let mut total = 0;
        let mut valid = 0;
        let mut expired = 0;
        let mut total_size = 0u64;

        let mut entries = tokio::fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                total += 1;
                if let Ok(metadata) = entry.metadata().await {
                    total_size += metadata.len();
                }
                if let Ok(data) = tokio::fs::read_to_string(&path).await
                    && let Ok(cache_entry) = serde_json::from_str::<CacheEntry>(&data)
                {
                    if cache_entry.is_valid(self.ttl.as_secs()) {
                        valid += 1;
                    } else {
                        expired += 1;
                    }
                }
            }
        }

        Ok(CacheStats {
            total_entries: total,
            valid_entries: valid,
            expired_entries: expired,
            total_size_bytes: total_size,
            cache_dir: self.cache_dir.clone(),
        })
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new().expect("Failed to initialize cache")
    }
}

/// Statistics about the cache.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
    pub total_size_bytes: u64,
    pub cache_dir: PathBuf,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache: {} entries ({} valid, {} expired), {} bytes at {}",
            self.total_entries,
            self.valid_entries,
            self.expired_entries,
            self.total_size_bytes,
            self.cache_dir.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecow::EcoString;
    use tempfile::TempDir;

    fn test_cache(ttl_secs: u64) -> (Cache, TempDir) {
        let temp_dir = TempDir::new().expect("create temp dir");
        let cache = Cache {
            cache_dir: temp_dir.path().to_path_buf(),
            ttl: Duration::from_secs(ttl_secs),
        };
        (cache, temp_dir)
    }

    #[test]
    fn test_cache_entry_validity() {
        let cmd = Command::new(EcoString::from("test"));
        let entry = CacheEntry::new(cmd.clone(), 12345);

        // Should be valid with a long TTL
        assert!(entry.is_valid(3600));

        // Should be invalid with zero TTL
        assert!(!entry.is_valid(0));
    }

    #[test]
    fn test_cache_entry_content_match() {
        let cmd = Command::new(EcoString::from("test"));
        let entry = CacheEntry::new(cmd, 12345);

        assert!(entry.matches_content(12345));
        assert!(!entry.matches_content(54321));
    }

    #[test]
    fn test_cache_key_generation() {
        let key1 = Cache::cache_key("git", None);
        assert_eq!(key1.as_str(), "git");

        let key2 = Cache::cache_key("git", Some("--help"));
        assert!(key2.starts_with("git_"));
        assert!(key2.len() > 4); // Has hash suffix
    }

    #[test]
    fn test_cache_key_sanitizes_paths() {
        let key = Cache::cache_key("path/to/command", None);
        assert!(!key.contains('/'));
    }

    #[tokio::test]
    async fn test_cache_roundtrip() {
        let (cache, _temp) = test_cache(3600);

        let mut cmd = Command::new(EcoString::from("mycmd"));
        cmd.description = EcoString::from("My command");
        cmd.usage = EcoString::from("mycmd [OPTIONS]");

        let content = "USAGE: mycmd [OPTIONS]\n\n-v  verbose";
        let hash = Cache::hash_content(content);

        // Store
        cache
            .set("mycmd", Some("--help"), hash, &cmd)
            .await
            .expect("cache set");

        // Retrieve
        let cached = cache.get("mycmd", Some("--help"), hash).await;
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.name.as_str(), "mycmd");
        assert_eq!(cached.description.as_str(), "My command");
    }

    #[tokio::test]
    async fn test_cache_miss_on_content_change() {
        let (cache, _temp) = test_cache(3600);

        let cmd = Command::new(EcoString::from("mycmd"));
        let content1 = "help text v1";
        let content2 = "help text v2";
        let hash1 = Cache::hash_content(content1);
        let hash2 = Cache::hash_content(content2);

        cache
            .set("mycmd", None, hash1, &cmd)
            .await
            .expect("cache set");

        // Same hash should hit
        assert!(cache.get("mycmd", None, hash1).await.is_some());

        // Different hash should miss
        assert!(cache.get("mycmd", None, hash2).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let (cache, _temp) = test_cache(0); // Zero TTL = immediate expiration

        let cmd = Command::new(EcoString::from("mycmd"));
        let hash = 12345;

        cache
            .set("mycmd", None, hash, &cmd)
            .await
            .expect("cache set");

        // Should miss due to expiration
        assert!(cache.get("mycmd", None, hash).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let (cache, _temp) = test_cache(3600);

        let cmd = Command::new(EcoString::from("cmd"));
        cache.set("cmd1", None, 1, &cmd).await.expect("set 1");
        cache.set("cmd2", None, 2, &cmd).await.expect("set 2");
        cache.set("cmd3", None, 3, &cmd).await.expect("set 3");

        let cleared = cache.clear().await.expect("clear");
        assert_eq!(cleared, 3);

        assert!(cache.get("cmd1", None, 1).await.is_none());
        assert!(cache.get("cmd2", None, 2).await.is_none());
        assert!(cache.get("cmd3", None, 3).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let (cache, _temp) = test_cache(3600);

        let cmd = Command::new(EcoString::from("cmd"));
        cache.set("cmd1", None, 1, &cmd).await.expect("set");
        cache.set("cmd2", None, 2, &cmd).await.expect("set");

        let stats = cache.stats().await.expect("stats");
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.valid_entries, 2);
        assert_eq!(stats.expired_entries, 0);
        assert!(stats.total_size_bytes > 0);
    }

    #[test]
    fn test_hash_content_deterministic() {
        let content = "some help text";
        let hash1 = Cache::hash_content(content);
        let hash2 = Cache::hash_content(content);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_content_different() {
        let hash1 = Cache::hash_content("content a");
        let hash2 = Cache::hash_content("content b");
        assert_ne!(hash1, hash2);
    }
}
