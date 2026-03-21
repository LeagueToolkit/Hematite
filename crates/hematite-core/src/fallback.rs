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
pub struct AssetFallback {
    /// Available asset paths in the WAD.
    available_paths: Vec<String>,
    /// Minimum similarity threshold (0.0 to 1.0). Default: 0.8
    min_similarity: f64,
    /// Cache of already-resolved fallbacks.
    cache: HashMap<String, Option<String>>,
}

impl AssetFallback {
    pub fn new(available_paths: Vec<String>, min_similarity: f64) -> Self {
        Self {
            available_paths,
            min_similarity,
            cache: HashMap::new(),
        }
    }

    /// Find the best fallback for a missing asset path.
    ///
    /// Returns `None` if no sufficiently similar asset exists.
    pub fn find_fallback(&mut self, missing_path: &str) -> Option<&str> {
        if self.cache.contains_key(missing_path) {
            return self.cache[missing_path].as_deref();
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
        self.cache.insert(missing_path.to_string(), result);
        self.cache[missing_path].as_deref()
    }
}

/// Extract file extension from a path (lowercase).
fn extract_extension(path: &str) -> String {
    path.rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase()
}
