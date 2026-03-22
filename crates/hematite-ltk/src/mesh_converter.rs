//! SCO ↔ SCB mesh format conversion using LTK.
//!
//! Converts between League's ASCII (.sco) and binary (.scb) static mesh formats.

use anyhow::{Context, Result};
use std::io::{BufReader, Cursor};

/// Convert SCO (ASCII) static mesh to SCB (binary) format.
///
/// Reads an ASCII static mesh file (.sco) and converts it to the binary
/// format (.scb) used in League of Legends WAD files.
///
/// ## Format Details
/// - **SCO**: ASCII text format with [ObjectBegin]/[ObjectEnd] markers
/// - **SCB**: Binary format with "r3d2Mesh" magic (version 3.2)
/// - Both formats support: vertices, faces, vertex colors, face colors
///
/// ## Process
/// 1. Parse ASCII .sco file → StaticMesh
/// 2. Serialize StaticMesh → binary .scb bytes
///
/// This is a **lossless** conversion (both formats store the same data).
pub fn sco_to_scb(sco_bytes: &[u8]) -> Result<Vec<u8>> {
    use league_toolkit::mesh::StaticMesh;

    // Parse SCO (ASCII format)
    let mut reader = BufReader::new(Cursor::new(sco_bytes));
    let mesh = StaticMesh::from_ascii(&mut reader)
        .context("Failed to parse SCO file")?;

    tracing::debug!(
        "Converting SCO→SCB: mesh '{}', {} vertices, {} faces",
        mesh.name(),
        mesh.vertices().len(),
        mesh.faces().len()
    );

    // Serialize to SCB (binary format)
    let mut output = Vec::new();
    mesh.to_writer(&mut output)
        .context("Failed to write SCB data")?;

    tracing::info!(
        "Converted SCO→SCB: mesh '{}', {} vertices, {} faces, {} bytes → {} bytes",
        mesh.name(),
        mesh.vertices().len(),
        mesh.faces().len(),
        sco_bytes.len(),
        output.len()
    );

    Ok(output)
}

/// Convert SCB (binary) static mesh to SCO (ASCII) format.
///
/// Reads a binary static mesh file (.scb) and converts it to the ASCII
/// format (.sco) used for editing and version control.
///
/// ## Process
/// 1. Parse binary .scb file → StaticMesh
/// 2. Serialize StaticMesh → ASCII .sco bytes
///
/// This is a **lossless** conversion (both formats store the same data).
pub fn scb_to_sco(scb_bytes: &[u8]) -> Result<Vec<u8>> {
    use league_toolkit::mesh::StaticMesh;

    // Parse SCB (binary format)
    let mut cursor = Cursor::new(scb_bytes);
    let mesh = StaticMesh::from_reader(&mut cursor)
        .context("Failed to parse SCB file")?;

    tracing::debug!(
        "Converting SCB→SCO: mesh '{}', {} vertices, {} faces",
        mesh.name(),
        mesh.vertices().len(),
        mesh.faces().len()
    );

    // Serialize to SCO (ASCII format)
    let mut output = Vec::new();
    mesh.to_ascii(&mut output)
        .context("Failed to write SCO data")?;

    tracing::info!(
        "Converted SCB→SCO: mesh '{}', {} vertices, {} faces, {} bytes → {} bytes",
        mesh.name(),
        mesh.vertices().len(),
        mesh.faces().len(),
        scb_bytes.len(),
        output.len()
    );

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires actual SCO/SCB files
    fn test_sco_to_scb_roundtrip() {
        // Test roundtrip: SCO → SCB → SCO should preserve data
        // This would require sample files for testing
    }

    #[test]
    #[ignore] // Requires actual SCO/SCB files
    fn test_scb_to_sco_conversion() {
        // Test SCB → SCO conversion with real files
    }
}
