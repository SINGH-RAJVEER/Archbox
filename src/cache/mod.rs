use crate::{package::Package, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub package: Package,
    pub cached_at: chrono::DateTime<chrono::Utc>,
    pub access_count: u64,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheIndex {
    pub entries: HashMap<String, CacheEntry>,
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct CacheManager {
    cache_dir: PathBuf,
    index: CacheIndex,
    max_size_mb: u64,
    ttl_hours: u64,
}

impl CacheManager {
    pub async fn new(cache_dir: PathBuf, max_size_mb: u64, ttl_hours: u64) -> Result<Self> {
        fs::create_dir_all(&cache_dir).await?;
        
        let index_path = cache_dir.join("index.json");
        let index = if index_path.exists() {
            let content = fs::read_to_string(&index_path).await?;
            serde_json::from_str(&content).unwrap_or_else(|_| CacheIndex::new())
        } else {
            CacheIndex::new()
        };
        
        let mut manager = Self {
            cache_dir,
            index,
            max_size_mb,
            ttl_hours,
        };
        
        manager.cleanup_expired().await?;
        Ok(manager)
    }
    
    pub async fn get_package(&mut self, name: &str) -> Option<Package> {
        if let Some(entry) = self.index.entries.get_mut(name) {
            // Check if cache entry is still valid
            let now = chrono::Utc::now();
            let age = now.signed_duration_since(entry.cached_at);
            
            if age.num_hours() < self.ttl_hours as i64 {
                entry.access_count += 1;
                entry.last_accessed = now;
                debug!("Cache hit for package: {}", name);
                return Some(entry.package.clone());
            } else {
                debug!("Cache expired for package: {}", name);
                self.index.entries.remove(name);
            }
        }
        
        debug!("Cache miss for package: {}", name);
        None
    }
    
    pub async fn store_package(&mut self, package: Package) -> Result<()> {
        let now = chrono::Utc::now();
        let entry = CacheEntry {
            package: package.clone(),
            cached_at: now,
            access_count: 1,
            last_accessed: now,
        };
        
        self.index.entries.insert(package.name.clone(), entry);
        self.save_index().await?;
        
        debug!("Cached package: {}", package.name);
        Ok(())
    }
    
    pub async fn invalidate(&mut self, name: &str) -> Result<()> {
        self.index.entries.remove(name);
        self.save_index().await?;
        info!("Invalidated cache for package: {}", name);
        Ok(())
    }
    
    pub async fn clear(&mut self) -> Result<()> {
        self.index.entries.clear();
        self.save_index().await?;
        
        // Remove cached files
        let mut entries = fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_name() != "index.json" {
                fs::remove_file(entry.path()).await?;
            }
        }
        
        info!("Cleared all cache");
        Ok(())
    }
    
    async fn cleanup_expired(&mut self) -> Result<()> {
        let now = chrono::Utc::now();
        let mut to_remove = Vec::new();
        
        for (name, entry) in &self.index.entries {
            let age = now.signed_duration_since(entry.cached_at);
            if age.num_hours() >= self.ttl_hours as i64 {
                to_remove.push(name.clone());
            }
        }
        
        for name in to_remove {
            self.index.entries.remove(&name);
        }
        
        if !self.index.entries.is_empty() {
            self.save_index().await?;
        }
        
        Ok(())
    }
    
    async fn save_index(&self) -> Result<()> {
        let index_path = self.cache_dir.join("index.json");
        let content = serde_json::to_string_pretty(&self.index)?;
        fs::write(&index_path, content).await?;
        Ok(())
    }
    
    pub fn get_statistics(&self) -> CacheStats {
        let total_entries = self.index.entries.len();
        let total_access_count: u64 = self.index.entries.values()
            .map(|e| e.access_count)
            .sum();
        
        let hit_rate = if total_access_count > 0 {
            (total_access_count as f64 / (total_access_count + total_entries as u64) as f64) * 100.0
        } else {
            0.0
        };
        
        CacheStats {
            total_entries,
            total_access_count,
            hit_rate,
            cache_size_mb: self.estimate_cache_size(),
        }
    }
    
    fn estimate_cache_size(&self) -> f64 {
        // Rough estimation based on serialized data
        let index_size = serde_json::to_string(&self.index)
            .map(|s| s.len())
            .unwrap_or(0);
        
        (index_size as f64) / (1024.0 * 1024.0)
    }
}

impl CacheIndex {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: chrono::Utc::now(),
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_access_count: u64,
    pub hit_rate: f64,
    pub cache_size_mb: f64,
}