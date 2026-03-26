//! Remote configuration fetching from GitHub.
//!
//! Fetches fix_config.json and champion_list.json from the Hematite repository,
//! caches them locally, and provides fallback to embedded configs.

use anyhow::{Context, Result};
use hematite_types::champion::ChampionList;
use hematite_types::config::FixConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// GitHub raw content URL base
const GITHUB_RAW_BASE: &str = "https://raw.githubusercontent.com/LeagueToolkit/Hematite/main/config";

/// URLs for remote configs
const FIX_CONFIG_URL: &str = const_format::formatcp!("{}/fix_config.json", GITHUB_RAW_BASE);
const CHAMPION_LIST_URL: &str = const_format::formatcp!("{}/champion_list.json", GITHUB_RAW_BASE);

/// Cache time-to-live: 1 hour
const CACHE_TTL: Duration = Duration::from_secs(3600);

/// HTTP request timeout: 10 seconds
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Get the cache directory path.
///
/// Uses `%APPDATA%\Hematite\cache` on Windows.
fn get_cache_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").context("APPDATA environment variable not set")?;
        Ok(PathBuf::from(appdata).join("Hematite").join("cache"))
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join("Library/Application Support/Hematite/cache"))
    }

    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(".config/hematite/cache"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Ok(PathBuf::from(".hematite/cache"))
    }
}

/// Check if a cached file is still valid (within TTL).
fn is_cache_valid(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    let Ok(modified) = metadata.modified() else {
        return false;
    };

    let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
        return false;
    };

    elapsed < CACHE_TTL
}

/// Fetch a JSON file from a URL with timeout.
fn fetch_json(url: &str) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent("Hematite/0.2")
        .build()
        .context("Failed to create HTTP client")?;

    let response = client.get(url).send().context("HTTP request failed")?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP {} from {}", response.status(), url);
    }

    response.text().context("Failed to read response body")
}

/// Load fix configuration with caching and remote fetch.
///
/// Strategy:
/// 1. If cache is valid (< 1 hour old), use cached config
/// 2. Try to fetch from GitHub
/// 3. If fetch succeeds, update cache
/// 4. If fetch fails, use stale cache (if available)
/// 5. If no cache, use embedded config
pub fn load_fix_config() -> FixConfig {
    let cache_dir = match get_cache_dir() {
        Ok(dir) => dir,
        Err(e) => {
            tracing::warn!("Failed to get cache directory: {e}. Using embedded config.");
            return load_embedded_fix_config();
        }
    };

    let cache_file = cache_dir.join("fix_config.json");

    // Try cached config first
    if is_cache_valid(&cache_file) {
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(config) = serde_json::from_str::<FixConfig>(&content) {
                tracing::debug!("Using cached fix config (version {})", config.version);
                return config;
            }
        }
    }

    // Try to fetch from GitHub
    tracing::info!("Fetching latest fix config from GitHub...");
    match fetch_json(FIX_CONFIG_URL) {
        Ok(content) => {
            match serde_json::from_str::<FixConfig>(&content) {
                Ok(config) => {
                    tracing::info!("Fetched fix config version {} from GitHub", config.version);

                    // Cache the fetched config
                    if let Err(e) = fs::create_dir_all(&cache_dir) {
                        tracing::warn!("Failed to create cache directory: {e}");
                    } else if let Err(e) = fs::write(&cache_file, &content) {
                        tracing::warn!("Failed to write cache file: {e}");
                    } else {
                        tracing::debug!("Cached fix config to {}", cache_file.display());
                    }

                    return config;
                }
                Err(e) => {
                    tracing::warn!("Failed to parse fetched config: {e}");
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to fetch config from GitHub: {e}");
        }
    }

    // Try stale cache as fallback
    if cache_file.exists() {
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(config) = serde_json::from_str::<FixConfig>(&content) {
                tracing::info!("Using stale cached config (version {})", config.version);
                return config;
            }
        }
    }

    // Final fallback: embedded config
    tracing::info!("Using embedded fix config");
    load_embedded_fix_config()
}

/// Load champion list with caching and remote fetch.
pub fn load_champion_list() -> ChampionList {
    let cache_dir = match get_cache_dir() {
        Ok(dir) => dir,
        Err(e) => {
            tracing::warn!("Failed to get cache directory: {e}. Using embedded champion list.");
            return load_embedded_champion_list();
        }
    };

    let cache_file = cache_dir.join("champion_list.json");

    // Try cached list first
    if is_cache_valid(&cache_file) {
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(list) = serde_json::from_str::<ChampionList>(&content) {
                tracing::debug!("Using cached champion list (version {})", list.version);
                return list;
            }
        }
    }

    // Try to fetch from GitHub
    tracing::info!("Fetching latest champion list from GitHub...");
    match fetch_json(CHAMPION_LIST_URL) {
        Ok(content) => {
            match serde_json::from_str::<ChampionList>(&content) {
                Ok(list) => {
                    tracing::info!("Fetched champion list version {} from GitHub", list.version);

                    // Cache the fetched list
                    if let Err(e) = fs::create_dir_all(&cache_dir) {
                        tracing::warn!("Failed to create cache directory: {e}");
                    } else if let Err(e) = fs::write(&cache_file, &content) {
                        tracing::warn!("Failed to write cache file: {e}");
                    } else {
                        tracing::debug!("Cached champion list to {}", cache_file.display());
                    }

                    return list;
                }
                Err(e) => {
                    tracing::warn!("Failed to parse fetched champion list: {e}");
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to fetch champion list from GitHub: {e}");
        }
    }

    // Try stale cache as fallback
    if cache_file.exists() {
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(list) = serde_json::from_str::<ChampionList>(&content) {
                tracing::info!(
                    "Using stale cached champion list (version {})",
                    list.version
                );
                return list;
            }
        }
    }

    // Final fallback: embedded list
    tracing::info!("Using embedded champion list");
    load_embedded_champion_list()
}

/// Load embedded fix config (compile-time bundled).
fn load_embedded_fix_config() -> FixConfig {
    const EMBEDDED_CONFIG: &str = include_str!("../../../config/fix_config.json");

    serde_json::from_str(EMBEDDED_CONFIG)
        .expect("Embedded fix_config.json is invalid - this is a build error")
}

/// Load embedded champion list (compile-time bundled).
fn load_embedded_champion_list() -> ChampionList {
    const EMBEDDED_LIST: &str = include_str!("../../../config/champion_list.json");

    serde_json::from_str(EMBEDDED_LIST)
        .expect("Embedded champion_list.json is invalid - this is a build error")
}

/// Clear all cached configs (force fresh fetch on next run).
#[allow(dead_code)]
pub fn clear_cache() -> Result<()> {
    let cache_dir = get_cache_dir()?;

    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir).with_context(|| {
            format!("Failed to remove cache directory: {}", cache_dir.display())
        })?;
        tracing::info!("Cleared config cache at {}", cache_dir.display());
    }

    Ok(())
}

/// Get cache status information for display.
#[allow(dead_code)]
#[derive(Debug)]
pub struct CacheStatus {
    pub fix_config_cached: bool,
    pub fix_config_valid: bool,
    pub champion_list_cached: bool,
    pub champion_list_valid: bool,
}

#[allow(dead_code)]
pub fn get_cache_status() -> Result<CacheStatus> {
    let cache_dir = get_cache_dir()?;
    let fix_config_cache = cache_dir.join("fix_config.json");
    let champion_list_cache = cache_dir.join("champion_list.json");

    Ok(CacheStatus {
        fix_config_cached: fix_config_cache.exists(),
        fix_config_valid: is_cache_valid(&fix_config_cache),
        champion_list_cached: champion_list_cache.exists(),
        champion_list_valid: is_cache_valid(&champion_list_cache),
    })
}
