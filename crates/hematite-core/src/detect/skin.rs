//! Skin Number Detection Module
//!
//! Detects available skin numbers from BIN file paths and determines if a mod is "binless".
//! Useful for multi-skin processing and understanding mod structure.

use regex::Regex;
use std::collections::HashSet;

/// Skin detection result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinInfo {
    /// Champion name extracted from paths
    pub champion: String,
    /// Available skin numbers found (e.g., [0, 1, 27])
    pub skin_numbers: Vec<u32>,
    /// Whether this mod is binless (no character BIN files found)
    pub is_binless: bool,
    /// All BIN file paths analyzed
    pub bin_paths: Vec<String>,
}

impl SkinInfo {
    /// Create a new empty SkinInfo
    pub fn new(champion: String) -> Self {
        Self {
            champion,
            skin_numbers: Vec::new(),
            is_binless: true,
            bin_paths: Vec::new(),
        }
    }

    /// Check if this skin info contains any valid data
    pub fn is_empty(&self) -> bool {
        self.skin_numbers.is_empty() && self.bin_paths.is_empty()
    }

    /// Get the primary skin number (usually skin0 or the lowest numbered skin)
    pub fn primary_skin(&self) -> Option<u32> {
        self.skin_numbers.iter().min().copied()
    }
}

/// Skin number detector
#[derive(Debug)]
pub struct SkinDetector {
    /// Regex pattern for extracting skin numbers from paths
    /// Matches patterns like:
    /// - data/{champion}_skin{number}.bin
    /// - data/characters/{champion}/skins/skin{number}.bin
    /// - {champion}_skin{number}_*.bin
    skin_pattern: Regex,
}

impl SkinDetector {
    /// Create a new skin detector
    pub fn new() -> Self {
        // Pattern to match skin numbers in various formats
        let skin_pattern =
            Regex::new(r"(?i)skin(\d{1,2})").expect("BUG: hardcoded regex pattern is invalid");

        Self { skin_pattern }
    }

    /// Extract skin number from a file path
    ///
    /// # Arguments
    /// * `path` - File path to analyze (e.g., "data/lux_skin07.bin")
    ///
    /// # Returns
    /// Optional skin number if found
    ///
    /// # Example
    /// ```
    /// use hematite_core::detect::skin::SkinDetector;
    ///
    /// let detector = SkinDetector::new();
    /// assert_eq!(detector.extract_skin_number("data/lux_skin07.bin"), Some(7));
    /// assert_eq!(detector.extract_skin_number("data/lux_base.bin"), None);
    /// ```
    pub fn extract_skin_number(&self, path: &str) -> Option<u32> {
        self.skin_pattern
            .captures(path)
            .and_then(|cap| cap.get(1).and_then(|m| m.as_str().parse::<u32>().ok()))
    }

    /// Detect all available skins from a list of file paths
    ///
    /// # Arguments
    /// * `file_paths` - List of file paths to analyze
    /// * `champion` - Expected champion name (for filtering)
    ///
    /// # Returns
    /// SkinInfo with detected skin numbers and metadata
    ///
    /// # Example
    /// ```
    /// use hematite_core::detect::skin::SkinDetector;
    ///
    /// let detector = SkinDetector::new();
    /// let paths = vec![
    ///     "data/lux_skin00.bin",
    ///     "data/lux_skin07.bin",
    ///     "data/lux_skin27.bin",
    /// ];
    /// let info = detector.detect_skins(&paths, "lux");
    /// assert_eq!(info.skin_numbers, vec![0, 7, 27]);
    /// assert!(!info.is_binless);
    /// ```
    pub fn detect_skins(&self, file_paths: &[impl AsRef<str>], champion: &str) -> SkinInfo {
        let champion_lower = champion.to_lowercase();
        let mut skin_info = SkinInfo::new(champion_lower.clone());

        let mut found_skin_numbers = HashSet::new();
        let mut found_character_bin = false;

        for path in file_paths {
            let path = path.as_ref();
            let path_lower = path.to_lowercase();

            // Check if this is a BIN file related to our champion
            if !path_lower.ends_with(".bin") {
                continue;
            }

            if !path_lower.contains(&champion_lower) {
                continue;
            }

            skin_info.bin_paths.push(path.to_string());

            // Check if this is a character BIN (not just any BIN)
            // Character BINs typically match: {champion}_skin{N}.bin
            if path_lower.contains(&format!("{}_skin", champion_lower))
                || path_lower.contains(&format!("data/{}/skins/skin", champion_lower))
            {
                found_character_bin = true;

                // Extract skin number
                if let Some(skin_num) = self.extract_skin_number(path) {
                    found_skin_numbers.insert(skin_num);
                }
            }
        }

        // Convert HashSet to sorted Vec
        let mut skin_numbers: Vec<u32> = found_skin_numbers.into_iter().collect();
        skin_numbers.sort();

        skin_info.skin_numbers = skin_numbers;
        skin_info.is_binless = !found_character_bin;

        skin_info
    }

    /// Check if a file path represents a character skin BIN
    ///
    /// # Arguments
    /// * `path` - File path to check
    ///
    /// # Returns
    /// True if this is likely a character skin BIN file
    pub fn is_character_skin_bin(&self, path: &str) -> bool {
        let path_lower = path.to_lowercase();

        if !path_lower.ends_with(".bin") {
            return false;
        }

        // Match patterns like:
        // - data/{champion}_skin{N}.bin
        // - data/characters/{champion}/skins/skin{N}.bin
        // - {champion}_skin{N}_*.bin (for concat, StaticMat, etc.)

        if path_lower.contains("_skin") && self.skin_pattern.is_match(&path_lower) {
            return true;
        }

        if path_lower.contains("/skins/skin") && self.skin_pattern.is_match(&path_lower) {
            return true;
        }

        false
    }

    /// Generate expected BIN paths for a champion and skin number
    ///
    /// # Arguments
    /// * `champion` - Champion name
    /// * `skin_number` - Skin number
    ///
    /// # Returns
    /// Vector of expected BIN file paths
    ///
    /// # Example
    /// ```
    /// use hematite_core::detect::skin::SkinDetector;
    ///
    /// let detector = SkinDetector::new();
    /// let paths = detector.generate_skin_paths("lux", 7);
    /// // Returns: ["data/lux_skin07.bin", "data/lux_skin07_concat.bin", ...]
    /// ```
    pub fn generate_skin_paths(&self, champion: &str, skin_number: u32) -> Vec<String> {
        let champion_lower = champion.to_lowercase();
        let skin_str = format!("{:02}", skin_number); // Zero-padded (07, 27, etc.)

        vec![
            format!("data/{}_skin{}.bin", champion_lower, skin_str),
            format!("data/{}_skin{}_concat.bin", champion_lower, skin_str),
            format!("data/{}_skin{}_StaticMat.bin", champion_lower, skin_str),
            format!(
                "data/characters/{}/skins/skin{}.bin",
                champion_lower, skin_number
            ),
        ]
    }
}

impl Default for SkinDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_skin_number() {
        let detector = SkinDetector::new();

        assert_eq!(detector.extract_skin_number("data/lux_skin07.bin"), Some(7));
        assert_eq!(detector.extract_skin_number("data/lux_skin00.bin"), Some(0));
        assert_eq!(
            detector.extract_skin_number("data/lux_skin27.bin"),
            Some(27)
        );
        assert_eq!(
            detector.extract_skin_number("data/characters/ahri/skins/skin09.bin"),
            Some(9)
        );
        assert_eq!(detector.extract_skin_number("data/lux_base.bin"), None);
        assert_eq!(detector.extract_skin_number("data/random.bin"), None);
    }

    #[test]
    fn test_detect_skins() {
        let detector = SkinDetector::new();

        let paths = vec![
            "data/lux_skin00.bin",
            "data/lux_skin07.bin",
            "data/lux_skin27.bin",
            "data/lux_skin07_concat.bin",
            "data/ahri_skin09.bin", // Different champion
        ];

        let info = detector.detect_skins(&paths, "lux");

        assert_eq!(info.champion, "lux");
        assert_eq!(info.skin_numbers, vec![0, 7, 27]);
        assert!(!info.is_binless);
        assert_eq!(info.bin_paths.len(), 4); // Only lux BINs
    }

    #[test]
    fn test_binless_detection() {
        let detector = SkinDetector::new();

        // Mod with only texture files, no character BINs
        let paths = vec!["data/lux/loadscreen.dds", "data/lux/textures/skin07.dds"];

        let info = detector.detect_skins(&paths, "lux");

        assert!(info.is_binless);
        assert!(info.skin_numbers.is_empty());
    }

    #[test]
    fn test_is_character_skin_bin() {
        let detector = SkinDetector::new();

        assert!(detector.is_character_skin_bin("data/lux_skin07.bin"));
        assert!(detector.is_character_skin_bin("data/lux_skin07_concat.bin"));
        assert!(detector.is_character_skin_bin("data/characters/ahri/skins/skin09.bin"));

        assert!(!detector.is_character_skin_bin("data/lux_base.bin"));
        assert!(!detector.is_character_skin_bin("data/random.bin"));
        assert!(!detector.is_character_skin_bin("data/lux.dds"));
    }

    #[test]
    fn test_generate_skin_paths() {
        let detector = SkinDetector::new();

        let paths = detector.generate_skin_paths("lux", 7);

        assert!(paths.contains(&"data/lux_skin07.bin".to_string()));
        assert!(paths.contains(&"data/lux_skin07_concat.bin".to_string()));
        assert!(paths.contains(&"data/lux_skin07_StaticMat.bin".to_string()));
        assert!(paths.contains(&"data/characters/lux/skins/skin7.bin".to_string()));
    }

    #[test]
    fn test_primary_skin() {
        let detector = SkinDetector::new();

        let paths = vec![
            "data/lux_skin07.bin",
            "data/lux_skin27.bin",
            "data/lux_skin00.bin",
        ];

        let info = detector.detect_skins(&paths, "lux");

        // Primary skin should be the lowest number (skin0)
        assert_eq!(info.primary_skin(), Some(0));
    }

    #[test]
    fn test_case_insensitivity() {
        let detector = SkinDetector::new();

        assert_eq!(detector.extract_skin_number("DATA/LUX_SKIN07.BIN"), Some(7));
        assert_eq!(detector.extract_skin_number("Data/Lux_Skin07.bin"), Some(7));
    }

    #[test]
    fn test_empty_skin_info() {
        let info = SkinInfo::new("lux".to_string());
        assert!(info.is_empty());
        assert_eq!(info.primary_skin(), None);
    }

    #[test]
    fn test_detect_skins_generic() {
        let detector = SkinDetector::new();

        // Test with String references
        let paths: Vec<String> = vec![
            "data/lux_skin00.bin".to_string(),
            "data/lux_skin07.bin".to_string(),
        ];

        let info = detector.detect_skins(&paths, "lux");
        assert_eq!(info.skin_numbers, vec![0, 7]);
    }

    #[test]
    fn test_non_zero_padded_paths() {
        let detector = SkinDetector::new();

        // Test paths with single digit (no zero padding)
        assert_eq!(detector.extract_skin_number("data/lux_skin7.bin"), Some(7));
        assert_eq!(
            detector.extract_skin_number("data/characters/ahri/skins/skin9.bin"),
            Some(9)
        );
    }
}
