//! Shader Hash Verification Module
//!
//! Validates that shader files referenced in BIN files actually exist in the game.
//! Missing shaders cause invisible models, so this module helps detect and fix those issues.
//!
//! Shader hashes are loaded from `hashes.shaders.txt` in the RitoShark hash directory.

use anyhow::{Context, Result};
use std::collections::HashSet;
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
}

impl ShaderValidator {
    /// Create an empty shader validator
    pub fn new() -> Self {
        Self {
            valid_shader_hashes: HashSet::new(),
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
            // Gracefully return empty validator if file doesn't exist
            return Ok(Self::new());
        }

        let file = File::open(&shader_file).with_context(|| {
            format!("Failed to open shader hash file: {}", shader_file.display())
        })?;
        let reader = BufReader::new(file);

        let mut valid_shader_hashes = HashSet::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue; // Skip empty lines and comments
            }

            // Format: "<hex_hash> <shader_path>" or just "<hex_hash>"
            let hash_str =
                if let Some((hash_part, _)) = line.split_once(|c: char| c.is_whitespace()) {
                    hash_part
                } else {
                    line
                };

            // Parse hex hash (with or without 0x prefix)
            let hash_str = hash_str.trim_start_matches("0x");
            if let Ok(hash) = u64::from_str_radix(hash_str, 16) {
                valid_shader_hashes.insert(hash);
            }
            // Skip invalid hashes silently
        }

        Ok(Self {
            valid_shader_hashes,
        })
    }

    /// Check if a shader hash is valid (exists in game data)
    ///
    /// # Arguments
    /// * `shader_hash` - The xxhash64 of the shader file path
    ///
    /// # Returns
    /// True if the shader exists in the game, false otherwise
    pub fn is_valid_shader(&self, shader_hash: u64) -> bool {
        self.valid_shader_hashes.contains(&shader_hash)
    }

    /// Validate a list of shader hashes
    ///
    /// # Arguments
    /// * `shader_hashes` - List of shader hashes to validate
    ///
    /// # Returns
    /// Vec of invalid shader hashes
    pub fn find_invalid_shaders(&self, shader_hashes: &[u64]) -> Vec<u64> {
        shader_hashes
            .iter()
            .filter(|&&hash| !self.is_valid_shader(hash))
            .copied()
            .collect()
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
        // Should not panic if APPDATA exists
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

        // All shaders are invalid when validator is empty
        assert_eq!(invalid.len(), 2);
    }

    #[test]
    fn test_valid_percentage_edge_cases() {
        // All invalid
        let result = ShaderValidationResult::new(5, vec![0x1, 0x2, 0x3, 0x4, 0x5]);
        assert_eq!(result.valid_percentage(), 0.0);

        // Half valid
        let result = ShaderValidationResult::new(10, vec![0x1, 0x2, 0x3, 0x4, 0x5]);
        assert_eq!(result.valid_percentage(), 50.0);
    }
}
