//! BNK Audio File Version Parser
//!
//! Parses BNK (Wwise SoundBank) files to extract version information
//! from the BKHD (Bank Header) section.
//!
//! Version handling (matches TopazModFixer):
//! - Versions < current are OLD and flagged for removal (e.g., 134 from older patches)
//! - Current version should be kept (configurable via JSON)
//! - Versions > current indicate the tool may need updating

/// Information about a parsed BNK file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BnkInfo {
    /// Parsed version from BKHD section
    pub version: Option<u32>,
    /// Whether this BNK should be removed based on version
    pub should_remove: bool,
    /// Reason for the decision
    pub reason: String,
}

/// Parse BNK data to extract version from BKHD section
///
/// BNK file structure:
/// - Multiple sections, each starting with a 4-byte ID and 4-byte length
/// - BKHD (Bank Header) section contains version at offset 0 of its data
///
/// # Arguments
/// * `data` - Raw BNK file bytes
/// * `min_version` - Minimum allowed version (versions below this are removed)
///
/// # Returns
/// * `BnkInfo` with version and removal decision
pub fn parse_bnk_version(data: &[u8], min_version: u32) -> BnkInfo {
    // Need at least 12 bytes: section ID (4) + length (4) + version (4)
    if data.len() < 12 {
        return BnkInfo {
            version: None,
            should_remove: true,
            reason: "File too small to parse".to_string(),
        };
    }

    let mut offset = 0;

    while offset < data.len().saturating_sub(8) {
        // Read section ID (4 bytes ASCII)
        let section_id = &data[offset..offset + 4];
        let section_id_str = std::str::from_utf8(section_id).unwrap_or("");

        // Read section length (4 bytes, little-endian)
        let section_length = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;

        // Check if this is the BKHD section
        if section_id_str == "BKHD" {
            // Version is the first 4 bytes of BKHD data
            if offset + 12 <= data.len() {
                let version = u32::from_le_bytes([
                    data[offset + 8],
                    data[offset + 9],
                    data[offset + 10],
                    data[offset + 11],
                ]);

                let should_remove = version < min_version;
                let reason = if version < min_version {
                    format!("Version {} is outdated (minimum: {})", version, min_version)
                } else if version == min_version {
                    format!("Version {} is current", version)
                } else {
                    // version > min_version
                    format!(
                        "Version {} is newer than expected (tool may need update)",
                        version
                    )
                };

                return BnkInfo {
                    version: Some(version),
                    should_remove,
                    reason,
                };
            }
            break;
        }

        // Move to next section
        offset += 8 + section_length;

        // Safety check for malformed files
        if section_length == 0 || offset > data.len() {
            break;
        }
    }

    // BKHD section not found
    BnkInfo {
        version: None,
        should_remove: true,
        reason: "BKHD section not found - unknown format".to_string(),
    }
}

/// Check if a file extension indicates a BNK file
pub fn is_bnk_extension(extension: &str) -> bool {
    extension.eq_ignore_ascii_case("bnk")
}

/// Check if a path matches the events.bnk pattern
/// These are the problematic audio files that often break after patches
pub fn is_events_bnk_path(path: &str) -> bool {
    let path_lower = path.to_lowercase();

    // Common patterns for events.bnk files
    path_lower.ends_with("events.bnk")
        || path_lower.contains("/events.bnk")
        || path_lower.contains("\\events.bnk")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_data() {
        let data = vec![];
        let info = parse_bnk_version(&data, 145);
        assert!(info.version.is_none());
        assert!(info.should_remove);
    }

    #[test]
    fn test_parse_too_small() {
        let data = vec![0u8; 8];
        let info = parse_bnk_version(&data, 145);
        assert!(info.version.is_none());
        assert!(info.should_remove);
    }

    #[test]
    fn test_parse_outdated_bkhd_v134() {
        // Construct a minimal BKHD section with version 134 (OLD version)
        let mut data = Vec::new();

        // Section ID: "BKHD"
        data.extend_from_slice(b"BKHD");

        // Section length: 8 bytes (version + some data)
        data.extend_from_slice(&8u32.to_le_bytes());

        // Version: 134 (old patch version)
        data.extend_from_slice(&134u32.to_le_bytes());

        // Some padding
        data.extend_from_slice(&0u32.to_le_bytes());

        let info = parse_bnk_version(&data, 145);
        assert_eq!(info.version, Some(134));
        assert!(info.should_remove); // 134 is outdated and should be removed
        assert!(info.reason.contains("outdated"));
    }

    #[test]
    fn test_parse_current_bkhd_v145() {
        // Version 145 is the current version and should be kept
        let mut data = Vec::new();
        data.extend_from_slice(b"BKHD");
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&145u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());

        let info = parse_bnk_version(&data, 145);
        assert_eq!(info.version, Some(145));
        assert!(!info.should_remove); // 145 is current and should be kept
        assert!(info.reason.contains("current"));
    }

    #[test]
    fn test_parse_old_version() {
        // Test with an old version (100 < 145)
        let mut data = Vec::new();
        data.extend_from_slice(b"BKHD");
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&100u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());

        let info = parse_bnk_version(&data, 145);
        assert_eq!(info.version, Some(100));
        assert!(info.should_remove); // Old version should be removed
        assert!(info.reason.contains("outdated"));
    }

    #[test]
    fn test_parse_newer_version() {
        // Test with a newer version (150 > 145) - should keep but warn
        let mut data = Vec::new();
        data.extend_from_slice(b"BKHD");
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&150u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());

        let info = parse_bnk_version(&data, 145);
        assert_eq!(info.version, Some(150));
        assert!(!info.should_remove); // Newer versions are kept (just log a warning)
        assert!(info.reason.contains("newer"));
    }

    #[test]
    fn test_is_events_bnk_path() {
        assert!(is_events_bnk_path("assets/sounds/events.bnk"));
        assert!(is_events_bnk_path("some/path/events.bnk"));
        assert!(is_events_bnk_path("Events.bnk"));
        assert!(!is_events_bnk_path("assets/sounds/sfx.bnk"));
        assert!(!is_events_bnk_path("other.bnk"));
    }

    #[test]
    fn test_is_bnk_extension() {
        assert!(is_bnk_extension("bnk"));
        assert!(is_bnk_extension("BNK"));
        assert!(is_bnk_extension("Bnk"));
        assert!(!is_bnk_extension("bin"));
        assert!(!is_bnk_extension("wad"));
    }
}
