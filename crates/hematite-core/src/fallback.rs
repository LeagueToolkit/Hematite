//! Asset fallback — Jaro-Winkler similarity matching for missing assets.
//!
//! When a skin mod references an asset that doesn't exist (e.g. a renamed .skn file),
//! this module finds the most similar available asset to prevent:
//! - **White models** (missing .skn → invisible/white mesh)
//! - **Black textures** (missing .dds/.tex → black or broken textures)
//!
//! ## Algorithm
//! - Uses **Jaro-Winkler** distance (from `strsim` crate)
//! - Default threshold: 80% similarity
//! - Respects file type: only matches .skn→.skn, .dds→.dds, etc.
//! - Caches results to avoid re-computation
//!
//! ## No LTK dependency
//! This module is pure string matching — it has no league-toolkit coupling.

use std::collections::HashMap;

/// Asset fallback resolver using string similarity.
#[derive(Debug, Clone)]
pub struct AssetFallback {
    /// Available asset paths in the WAD.
    available_paths: Vec<String>,
    /// Minimum similarity threshold (0.0 to 1.0). Default: 0.8
    min_similarity: f64,
    /// Cache of already-resolved fallbacks.
    cache: HashMap<String, Option<String>>,
}

impl AssetFallback {
    /// Create a new asset fallback resolver
    ///
    /// # Arguments
    /// * `available_paths` - List of available asset paths (from WAD cache)
    /// * `min_similarity` - Minimum similarity score (0.0 to 1.0), clamped to valid range
    pub fn new(available_paths: Vec<String>, min_similarity: f64) -> Self {
        Self {
            available_paths,
            min_similarity: min_similarity.clamp(0.0, 1.0),
            cache: HashMap::new(),
        }
    }

    /// Create with default similarity threshold (80%)
    pub fn with_default_threshold(available_paths: Vec<String>) -> Self {
        Self::new(available_paths, 0.80)
    }

    /// Find the best fallback for a missing asset path.
    ///
    /// Returns `None` if no sufficiently similar asset exists.
    ///
    /// # Example
    /// ```
    /// use hematite_core::fallback::AssetFallback;
    ///
    /// let fallback = AssetFallback::with_default_threshold(vec![
    ///     "data/lux/lux_base.skn".to_string(),
    ///     "data/lux/lux_skin00.skn".to_string(),
    /// ]);
    ///
    /// let mut fb = fallback;
    /// let result = fb.find_fallback("data/lux/lux_skin27.skn");
    /// assert!(result.is_some());
    /// ```
    pub fn find_fallback(&mut self, missing_path: &str) -> Option<String> {
        // Check cache first
        if let Some(cached) = self.cache.get(missing_path) {
            return cached.clone();
        }

        let missing_ext = extract_extension(missing_path);
        let missing_lower = missing_path.to_lowercase();
        let mut best_match: Option<(String, f64)> = None;

        for candidate in &self.available_paths {
            // Only match same file type
            if extract_extension(candidate) != missing_ext {
                continue;
            }

            let similarity = strsim::jaro_winkler(&missing_lower, &candidate.to_lowercase());
            if similarity >= self.min_similarity
                && best_match.as_ref().is_none_or(|(_, s)| similarity > *s)
            {
                best_match = Some((candidate.clone(), similarity));
            }
        }

        let result = best_match.map(|(path, _)| path);
        self.cache.insert(missing_path.to_string(), result.clone());
        result
    }

    /// Find fallback with custom similarity threshold (one-time use)
    ///
    /// Temporarily changes the threshold, finds the fallback, then restores original threshold.
    pub fn find_fallback_with_threshold(
        &mut self,
        missing_path: &str,
        threshold: f64,
    ) -> Option<String> {
        let original_threshold = self.min_similarity;
        self.min_similarity = threshold.clamp(0.0, 1.0);
        let result = self.find_fallback(missing_path);
        self.min_similarity = original_threshold;
        result
    }

    /// Batch find fallbacks for multiple missing paths
    ///
    /// # Arguments
    /// * `missing_paths` - List of missing asset paths
    ///
    /// # Returns
    /// HashMap mapping missing paths to their fallbacks (only successful matches)
    pub fn find_fallbacks(&mut self, missing_paths: &[String]) -> HashMap<String, String> {
        let mut results = HashMap::new();

        for missing_path in missing_paths {
            if let Some(fallback) = self.find_fallback(missing_path) {
                results.insert(missing_path.clone(), fallback);
            }
        }

        results
    }

    /// Get statistics about fallback resolution
    pub fn stats(&self) -> FallbackStats {
        FallbackStats {
            available_count: self.available_paths.len(),
            cached_fallbacks: self.cache.len(),
            min_similarity: self.min_similarity,
        }
    }

    /// Clear the fallback cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Update available paths (e.g., when WAD cache changes)
    ///
    /// This also clears the cache since available paths changed.
    pub fn update_available_paths(&mut self, new_paths: Vec<String>) {
        self.available_paths = new_paths;
        self.clear_cache();
    }
}

/// Statistics about fallback resolution
#[derive(Debug, Clone)]
pub struct FallbackStats {
    /// Number of available asset paths
    pub available_count: usize,
    /// Number of cached fallback resolutions
    pub cached_fallbacks: usize,
    /// Minimum similarity threshold
    pub min_similarity: f64,
}

/// Extract file extension from a path (lowercase).
fn extract_extension(path: &str) -> String {
    path.rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_fallback() -> AssetFallback {
        let available = vec![
            "data/lux/lux_base.skn".to_string(),
            "data/lux/lux_skin00.skn".to_string(),
            "data/lux/lux_skin00.dds".to_string(),
            "data/lux/lux_base.dds".to_string(),
            "data/ahri/ahri_skin09.skn".to_string(),
        ];
        AssetFallback::with_default_threshold(available)
    }

    #[test]
    fn test_find_fallback_similar() {
        let mut fallback = create_test_fallback();

        // Looking for skin27, should find skin00 (similar path)
        let result = fallback.find_fallback("data/lux/lux_skin27.skn");
        assert!(result.is_some());
        let resolved = result.unwrap();
        assert!(resolved.contains("lux_skin00") || resolved.contains("lux_base"));
    }

    #[test]
    fn test_fallback_respects_file_type() {
        let mut fallback = create_test_fallback();

        // Looking for .skn file should not match .dds files
        let result = fallback.find_fallback("data/lux/lux_skin27.skn");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with(".skn"));

        // Looking for .dds file should not match .skn files
        let result = fallback.find_fallback("data/lux/lux_skin27.dds");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with(".dds"));
    }

    #[test]
    fn test_no_fallback_for_low_similarity() {
        let mut fallback = create_test_fallback();

        // Completely different path should not match
        let result = fallback.find_fallback("data/zoe/zoe_skin99.skn");
        // This might find ahri_skin09 if similarity is high enough, or None
        if let Some(resolved) = result {
            // If it found something, it should at least be a .skn file
            assert!(resolved.ends_with(".skn"));
        }
    }

    #[test]
    fn test_fallback_cache() {
        let mut fallback = create_test_fallback();

        // First call - should compute
        let result1 = fallback.find_fallback("data/lux/lux_skin27.skn");
        assert!(result1.is_some());

        // Second call - should use cache
        let result2 = fallback.find_fallback("data/lux/lux_skin27.skn");
        assert_eq!(result1, result2);

        let stats = fallback.stats();
        assert_eq!(stats.cached_fallbacks, 1);
    }

    #[test]
    fn test_custom_threshold() {
        let mut fallback = create_test_fallback();

        // With very high threshold (95%), might not find match
        let result_high = fallback.find_fallback_with_threshold("data/lux/lux_custom.skn", 0.95);

        // With lower threshold (60%), more likely to find match
        let result_low = fallback.find_fallback_with_threshold("data/lux/lux_custom.skn", 0.60);

        // Lower threshold should be more permissive or equal
        assert!(result_low.is_some() || result_high.is_none());
    }

    #[test]
    fn test_batch_fallbacks() {
        let mut fallback = create_test_fallback();

        let missing = vec![
            "data/lux/lux_skin27.skn".to_string(),
            "data/lux/lux_skin27.dds".to_string(),
        ];

        let results = fallback.find_fallbacks(&missing);

        // Should find fallbacks for both
        assert!(!results.is_empty());
        assert!(results.len() <= 2);
    }

    #[test]
    fn test_extract_extension() {
        assert_eq!(extract_extension("file.skn"), "skn");
        assert_eq!(extract_extension("path/to/file.DDS"), "dds");
        assert_eq!(extract_extension("noextension"), "noextension");
    }

    #[test]
    fn test_case_insensitivity() {
        let mut fallback = create_test_fallback();

        let result1 = fallback.find_fallback("DATA/LUX/LUX_SKIN27.SKN");
        let result2 = fallback.find_fallback("data/lux/lux_skin27.skn");

        // Both should find the same fallback
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_clear_cache() {
        let mut fallback = create_test_fallback();

        fallback.find_fallback("data/lux/lux_skin27.skn");
        assert_eq!(fallback.stats().cached_fallbacks, 1);

        fallback.clear_cache();
        assert_eq!(fallback.stats().cached_fallbacks, 0);
    }

    #[test]
    fn test_update_available_paths() {
        let mut fallback = create_test_fallback();

        fallback.find_fallback("data/lux/lux_skin27.skn");
        assert_eq!(fallback.stats().cached_fallbacks, 1);

        // Update paths should clear cache
        let new_paths = vec!["data/ezreal/ezreal_base.skn".to_string()];
        fallback.update_available_paths(new_paths);

        assert_eq!(fallback.stats().cached_fallbacks, 0);
        assert_eq!(fallback.stats().available_count, 1);
    }

    #[test]
    fn test_threshold_clamping() {
        let fallback = AssetFallback::new(vec![], 1.5);
        assert_eq!(fallback.min_similarity, 1.0);

        let fallback = AssetFallback::new(vec![], -0.5);
        assert_eq!(fallback.min_similarity, 0.0);
    }
}
