//! File format converters.
//!
//! Registry of converters for transforming file formats (DDS→TEX, SCO→SCB, etc.).

use anyhow::Result;
use std::collections::HashMap;

/// A file format converter function.
pub type ConverterFn = fn(&[u8]) -> Result<Vec<u8>>;

/// Registry of file format converters.
pub struct ConverterRegistry {
    converters: HashMap<String, ConverterFn>,
}

impl ConverterRegistry {
    /// Create a new converter registry with built-in converters.
    pub fn new() -> Self {
        let mut registry = Self {
            converters: HashMap::new(),
        };

        // Register built-in converters
        registry.register("dds_to_tex", dds_to_tex);
        registry.register("sco_to_scb", sco_to_scb);

        registry
    }

    /// Register a converter function.
    pub fn register(&mut self, name: &str, converter: ConverterFn) {
        self.converters.insert(name.to_string(), converter);
    }

    /// Convert a file using the specified converter.
    pub fn convert(&self, converter_name: &str, input: &[u8]) -> Result<Vec<u8>> {
        let converter = self.converters
            .get(converter_name)
            .ok_or_else(|| anyhow::anyhow!("Converter not found: {}", converter_name))?;

        converter(input)
    }
}

impl Default for ConverterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// BUILT-IN CONVERTERS
// ============================================================================

/// Convert DDS texture to TEX format.
///
/// DDS and TEX are essentially the same format for League, so this is a no-op
/// converter that just returns the input unchanged. The real fix is the extension
/// change in the file path.
fn dds_to_tex(input: &[u8]) -> Result<Vec<u8>> {
    // DDS and TEX have identical binary format in League
    // The fix is purely the file extension change
    Ok(input.to_vec())
}

/// Convert SCO (SimpleSkin/ComplexSkin old) to SCB (SimpleSkin/ComplexSkin binary) format.
///
/// TODO: Implement actual SCO→SCB conversion logic.
/// For now, this is a placeholder that returns the input unchanged.
fn sco_to_scb(input: &[u8]) -> Result<Vec<u8>> {
    // TODO: Implement SCO→SCB conversion
    // SCO is an older skin format, SCB is the newer binary format
    // For now, return unchanged as a placeholder
    tracing::warn!("SCO→SCB conversion not yet implemented, returning unchanged");
    Ok(input.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let registry = ConverterRegistry::new();
        assert!(registry.converters.contains_key("dds_to_tex"));
        assert!(registry.converters.contains_key("sco_to_scb"));
    }

    #[test]
    fn test_dds_to_tex() {
        let input = vec![0x44, 0x44, 0x53, 0x20]; // "DDS " magic
        let output = dds_to_tex(&input).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn test_convert() {
        let registry = ConverterRegistry::new();
        let input = vec![1, 2, 3, 4];
        let output = registry.convert("dds_to_tex", &input).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn test_missing_converter() {
        let registry = ConverterRegistry::new();
        let result = registry.convert("nonexistent", &[]);
        assert!(result.is_err());
    }
}
