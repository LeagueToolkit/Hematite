//! Shader Hash Verification Module
//!
//! Validates that shader files referenced in BIN files actually exist in the game.
//! Missing shaders cause invisible models, so this module helps detect and fix those issues.
//!
//! Shader hashes are loaded from `hashes.shaders.txt` in the RitoShark hash directory.
//!
//! ## Token-based fallback matching
//! When a shader reference is invalid, `find_closest_shader` uses token-based similarity
//! to find the best replacement. Tokens are extracted by splitting on `_` and normalizing
//! common misspellings (e.g. MultiLayered→MultiLayer, Addative→Additive).

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Shader hash validator
///
/// Validates that shader references in BIN files exist in the game data.
/// Uses shader hash list from RitoShark shared directory.
#[derive(Debug, Clone)]
pub struct ShaderValidator {
    /// Set of valid shader hashes (xxhash64)
    valid_shader_hashes: HashSet<u64>,
    /// Hash → shader path mapping (for token-based fallback matching)
    shader_paths: HashMap<u64, String>,
}

impl ShaderValidator {
    /// Create an empty shader validator
    pub fn new() -> Self {
        Self {
            valid_shader_hashes: HashSet::new(),
            shader_paths: HashMap::new(),
        }
    }

    /// Get the RitoShark hash directory path
    pub fn get_hash_dir() -> Result<PathBuf> {
        let appdata = std::env::var("APPDATA").context("APPDATA environment variable not set")?;
        Ok(PathBuf::from(appdata)
            .join("RitoShark")
            .join("Requirements")
            .join("Hashes"))
    }

    /// Load shader hashes from RitoShark shared directory
    ///
    /// Loads from `hashes.shaders.txt` which contains valid shader file hashes
    /// from the game's DATA/FINAL/ directories.
    ///
    /// Returns Ok with empty set if file doesn't exist (graceful fallback).
    pub fn load() -> Result<Self> {
        let hash_dir = Self::get_hash_dir()?;
        let shader_file = hash_dir.join("hashes.shaders.txt");

        if !shader_file.exists() {
            return Ok(Self::new());
        }

        let file = File::open(&shader_file).with_context(|| {
            format!("Failed to open shader hash file: {}", shader_file.display())
        })?;
        let reader = BufReader::new(file);

        let mut valid_shader_hashes = HashSet::new();
        let mut shader_paths = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Format: "<hex_hash> <shader_path>" or just "<hex_hash>"
            let (hash_str, path_opt) = if let Some((hash_part, path_part)) =
                line.split_once(|c: char| c.is_whitespace())
            {
                (hash_part, Some(path_part.trim().to_string()))
            } else {
                (line, None)
            };

            let hash_str = hash_str.trim_start_matches("0x");
            if let Ok(hash) = u64::from_str_radix(hash_str, 16) {
                valid_shader_hashes.insert(hash);
                if let Some(path) = path_opt {
                    shader_paths.insert(hash, path);
                }
            }
        }

        Ok(Self {
            valid_shader_hashes,
            shader_paths,
        })
    }

    /// Check if a shader hash is valid (exists in game data)
    pub fn is_valid_shader(&self, shader_hash: u64) -> bool {
        self.valid_shader_hashes.contains(&shader_hash)
    }

    /// Validate a list of shader hashes
    pub fn find_invalid_shaders(&self, shader_hashes: &[u64]) -> Vec<u64> {
        shader_hashes
            .iter()
            .filter(|&&hash| !self.is_valid_shader(hash))
            .copied()
            .collect()
    }

    /// Get the path for a shader hash (if loaded with paths)
    pub fn resolve_path(&self, hash: u64) -> Option<&str> {
        self.shader_paths.get(&hash).map(|s| s.as_str())
    }

    /// Get all valid shader paths for matching
    pub fn all_paths(&self) -> impl Iterator<Item = (u64, &str)> {
        self.shader_paths.iter().map(|(&h, p)| (h, p.as_str()))
    }

    /// Find the closest valid shader to a given invalid shader name using token similarity.
    ///
    /// Uses the algorithm from TopazModFixer:
    /// 1. Normalize misspellings (MultiLayered→MultiLayer, Addative→Additive)
    /// 2. Split by `_` into tokens
    /// 3. Maximize mutual token count, minimize non-mutual count
    pub fn find_closest_shader(&self, invalid_name: &str) -> Option<(String, u64)> {
        if self.shader_paths.is_empty() {
            return None;
        }

        let target_tokens = tokenize(invalid_name);
        if target_tokens.is_empty() {
            return None;
        }

        // Extract category from invalid name (first path segment after "data/")
        let invalid_lower = invalid_name.to_lowercase();
        let target_category = extract_category(&invalid_lower);

        let mut best_match: Option<(String, u64, usize, usize)> = None; // (path, hash, mutual, non_mutual)

        for (&hash, path) in &self.shader_paths {
            let path_lower = path.to_lowercase();
            let candidate_category = extract_category(&path_lower);

            // Prefer same-category matches
            if target_category != candidate_category {
                continue;
            }

            let candidate_tokens = tokenize(path);
            let mutual = count_mutual(&target_tokens, &candidate_tokens);
            let non_mutual = target_tokens.len() + candidate_tokens.len() - 2 * mutual;

            if mutual == 0 {
                continue;
            }

            let is_better = match &best_match {
                None => true,
                Some((_, _, best_mutual, best_non_mutual)) => {
                    mutual > *best_mutual
                        || (mutual == *best_mutual && non_mutual < *best_non_mutual)
                }
            };

            if is_better {
                best_match = Some((path.clone(), hash, mutual, non_mutual));
            }
        }

        // If no same-category match, try all shaders
        if best_match.is_none() {
            for (&hash, path) in &self.shader_paths {
                let candidate_tokens = tokenize(path);
                let mutual = count_mutual(&target_tokens, &candidate_tokens);
                let non_mutual = target_tokens.len() + candidate_tokens.len() - 2 * mutual;

                if mutual == 0 {
                    continue;
                }

                let is_better = match &best_match {
                    None => true,
                    Some((_, _, best_mutual, best_non_mutual)) => {
                        mutual > *best_mutual
                            || (mutual == *best_mutual && non_mutual < *best_non_mutual)
                    }
                };

                if is_better {
                    best_match = Some((path.clone(), hash, mutual, non_mutual));
                }
            }
        }

        best_match.map(|(path, hash, _, _)| (path, hash))
    }

    /// Get total number of known valid shaders
    pub fn shader_count(&self) -> usize {
        self.valid_shader_hashes.len()
    }

    /// Check if shader validation is available (has loaded any shaders)
    pub fn is_available(&self) -> bool {
        !self.valid_shader_hashes.is_empty()
    }
}

impl Default for ShaderValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize common misspellings in shader names.
fn normalize(name: &str) -> String {
    name.replace("MultiLayered", "MultiLayer")
        .replace("multilayered", "multilayer")
        .replace("Addative", "Additive")
        .replace("addative", "additive")
}

/// Tokenize a shader path for similarity matching.
/// Extracts the filename, normalizes misspellings, splits by `_`, lowercases.
fn tokenize(path: &str) -> Vec<String> {
    // Extract filename from path
    let name = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".bin");

    let normalized = normalize(name);
    normalized
        .split('_')
        .map(|t| t.to_lowercase())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Extract category from a shader path (e.g. "data/shaders/character" → "character").
fn extract_category(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 3 {
        parts[parts.len() - 2].to_string()
    } else {
        String::new()
    }
}

/// Count mutual tokens between two token lists.
fn count_mutual(a: &[String], b: &[String]) -> usize {
    let mut b_remaining: Vec<bool> = vec![true; b.len()];
    let mut count = 0;

    for token_a in a {
        for (i, token_b) in b.iter().enumerate() {
            if b_remaining[i] && token_a == token_b {
                b_remaining[i] = false;
                count += 1;
                break;
            }
        }
    }

    count
}

/// Shader validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderValidationResult {
    /// Total number of shader references checked
    pub total_checked: usize,
    /// Number of valid shader references
    pub valid_count: usize,
    /// Number of invalid/missing shader references
    pub invalid_count: usize,
    /// List of invalid shader hashes
    pub invalid_hashes: Vec<u64>,
}

impl ShaderValidationResult {
    /// Create a new validation result
    pub fn new(total: usize, invalid_hashes: Vec<u64>) -> Self {
        let invalid_count = invalid_hashes.len();
        let valid_count = total.saturating_sub(invalid_count);

        Self {
            total_checked: total,
            valid_count,
            invalid_count,
            invalid_hashes,
        }
    }

    /// Check if all shaders are valid
    pub fn all_valid(&self) -> bool {
        self.invalid_count == 0
    }

    /// Get percentage of valid shaders
    pub fn valid_percentage(&self) -> f64 {
        if self.total_checked == 0 {
            100.0
        } else {
            (self.valid_count as f64 / self.total_checked as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validator() {
        let validator = ShaderValidator::new();
        assert_eq!(validator.shader_count(), 0);
        assert!(!validator.is_available());
    }

    #[test]
    fn test_is_valid_shader() {
        let mut validator = ShaderValidator::new();
        validator.valid_shader_hashes.insert(0x1234567890ABCDEF);

        assert!(validator.is_valid_shader(0x1234567890ABCDEF));
        assert!(!validator.is_valid_shader(0xDEADBEEFCAFEBABE));
    }

    #[test]
    fn test_find_invalid_shaders() {
        let mut validator = ShaderValidator::new();
        validator.valid_shader_hashes.insert(0x1111);
        validator.valid_shader_hashes.insert(0x2222);

        let shaders = vec![0x1111, 0x2222, 0x3333, 0x4444];
        let invalid = validator.find_invalid_shaders(&shaders);

        assert_eq!(invalid.len(), 2);
        assert!(invalid.contains(&0x3333));
        assert!(invalid.contains(&0x4444));
    }

    #[test]
    fn test_validation_result() {
        let result = ShaderValidationResult::new(10, vec![0x1111, 0x2222]);

        assert_eq!(result.total_checked, 10);
        assert_eq!(result.valid_count, 8);
        assert_eq!(result.invalid_count, 2);
        assert!(!result.all_valid());
        assert_eq!(result.valid_percentage(), 80.0);
    }

    #[test]
    fn test_validation_result_all_valid() {
        let result = ShaderValidationResult::new(10, vec![]);

        assert!(result.all_valid());
        assert_eq!(result.valid_percentage(), 100.0);
    }

    #[test]
    fn test_validation_result_empty() {
        let result = ShaderValidationResult::new(0, vec![]);

        assert!(result.all_valid());
        assert_eq!(result.valid_percentage(), 100.0);
    }

    #[test]
    fn test_get_hash_dir() {
        if std::env::var("APPDATA").is_ok() {
            let dir = ShaderValidator::get_hash_dir();
            assert!(dir.is_ok());
            let path = dir.unwrap();
            assert!(path.to_string_lossy().contains("RitoShark"));
            assert!(path.to_string_lossy().contains("Hashes"));
        }
    }

    #[test]
    fn test_shader_count() {
        let mut validator = ShaderValidator::new();
        assert_eq!(validator.shader_count(), 0);

        validator.valid_shader_hashes.insert(0x1111);
        validator.valid_shader_hashes.insert(0x2222);
        assert_eq!(validator.shader_count(), 2);
    }

    #[test]
    fn test_find_invalid_shaders_empty() {
        let validator = ShaderValidator::new();
        let shaders = vec![0x1111, 0x2222];
        let invalid = validator.find_invalid_shaders(&shaders);

        assert_eq!(invalid.len(), 2);
    }

    #[test]
    fn test_valid_percentage_edge_cases() {
        let result = ShaderValidationResult::new(5, vec![0x1, 0x2, 0x3, 0x4, 0x5]);
        assert_eq!(result.valid_percentage(), 0.0);

        let result = ShaderValidationResult::new(10, vec![0x1, 0x2, 0x3, 0x4, 0x5]);
        assert_eq!(result.valid_percentage(), 50.0);
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("data/shaders/character/skin_MultiLayer_Opaque.bin");
        assert_eq!(tokens, vec!["skin", "multilayer", "opaque"]);
    }

    #[test]
    fn test_normalize_misspellings() {
        assert!(normalize("MultiLayered").contains("MultiLayer"));
        assert!(!normalize("MultiLayered").contains("MultiLayered"));
        assert!(normalize("Addative").contains("Additive"));
    }

    #[test]
    fn test_count_mutual() {
        let a = vec![
            "skin".to_string(),
            "opaque".to_string(),
            "character".to_string(),
        ];
        let b = vec!["skin".to_string(), "opaque".to_string(), "vfx".to_string()];
        assert_eq!(count_mutual(&a, &b), 2);
    }

    #[test]
    fn test_find_closest_shader() {
        let mut validator = ShaderValidator::new();
        validator.valid_shader_hashes.insert(0xAAA);
        validator.valid_shader_hashes.insert(0xBBB);
        validator.shader_paths.insert(
            0xAAA,
            "data/shaders/character/Skin_MultiLayer_Opaque".to_string(),
        );
        validator
            .shader_paths
            .insert(0xBBB, "data/shaders/character/Skin_Opaque".to_string());

        // "Skin_MultiLayered_Opaque" should match "Skin_MultiLayer_Opaque" (misspelling normalization)
        let result =
            validator.find_closest_shader("data/shaders/character/Skin_MultiLayered_Opaque");
        assert!(result.is_some());
        let (path, hash) = result.unwrap();
        assert_eq!(hash, 0xAAA);
        assert!(path.contains("MultiLayer"));
    }
}
