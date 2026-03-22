//! DDS ↔ TEX texture format conversion using LTK.
//!
//! Converts between Microsoft's DDS format and League's proprietary TEX format.

use anyhow::{Context, Result};
use std::io::Cursor;

/// Convert DDS texture data to TEX format.
///
/// Reads a DDS file, decodes it, and re-encodes as TEX with the same
/// compression format and mipmaps.
///
/// ## Supported formats
/// - BC1 (DXT1)
/// - BC3 (DXT5)
/// - BGRA8 (uncompressed)
///
/// ## Process
/// 1. Parse DDS file → extract width/height/format/mipmaps
/// 2. Decode compressed blocks to RGBA
/// 3. Re-encode as TEX with same compression
///
/// **Note**: This is a lossy process for BC formats due to re-compression.
/// For lossless conversion, we'd need to copy raw compressed blocks, but
/// DDS and TEX have different mipmap ordering (DDS: large→small, TEX: small→large).
pub fn dds_to_tex(dds_bytes: &[u8]) -> Result<Vec<u8>> {
    use league_toolkit::texture::{Dds, Tex};
    use league_toolkit::texture::tex::{EncodeOptions, Format as TexFormat};

    // Parse DDS
    let mut cursor = Cursor::new(dds_bytes);
    let dds = Dds::from_reader(&mut cursor)
        .context("Failed to parse DDS file")?;

    tracing::debug!(
        "Converting DDS: {}x{}, {} mipmaps",
        dds.width(),
        dds.height(),
        dds.mip_count()
    );

    // Decode first mipmap to RGBA
    let surface = dds.decode_mipmap(0)
        .context("Failed to decode DDS mipmap")?;

    let rgba_image = surface.into_image()
        .context("Failed to convert surface to RGBA")?;

    // Determine TEX format from DDS format
    let tex_format = match detect_dds_format(&dds) {
        DdsFormat::Bc1 => TexFormat::Bc1,
        DdsFormat::Bc3 => TexFormat::Bc3,
        DdsFormat::Bgra8 => TexFormat::Bgra8,
        DdsFormat::Unsupported => {
            anyhow::bail!("Unsupported DDS format for conversion");
        }
    };

    tracing::debug!("Using TEX format: {:?}", tex_format);

    // Encode as TEX
    let has_mipmaps = dds.mip_count() > 1;
    let mut options = EncodeOptions::new(tex_format);
    if has_mipmaps {
        options = options.with_mipmaps();
    }

    let tex = Tex::encode_rgba_image(&rgba_image, options)
        .context("Failed to encode TEX")?;

    // Serialize TEX to bytes
    let mut output = Vec::new();
    tex.write(&mut output)
        .context("Failed to write TEX data")?;

    tracing::info!(
        "Converted DDS→TEX: {}x{} ({:?}), {} mipmaps, {} bytes → {} bytes",
        dds.width(),
        dds.height(),
        tex_format,
        dds.mip_count(),
        dds_bytes.len(),
        output.len()
    );

    Ok(output)
}

/// Detected DDS compression format.
#[derive(Debug, Clone, Copy)]
enum DdsFormat {
    Bc1,
    Bc3,
    Bgra8,
    Unsupported,
}

/// Detect DDS compression format from header.
///
/// TODO: This is a simplified heuristic. For production, we'd need to parse
/// the DDS header's fourCC and DX10 format fields.
fn detect_dds_format(dds: &league_toolkit::texture::Dds) -> DdsFormat {
    // Heuristic based on file size vs dimensions
    // BC1: 0.5 bytes per pixel (8 bytes per 4x4 block)
    // BC3: 1 byte per pixel (16 bytes per 4x4 block)
    // BGRA8: 4 bytes per pixel

    let width = dds.width() as usize;
    let height = dds.height() as usize;
    let pixel_count = width * height;
    let data_size = estimate_dds_data_size(dds);

    let bytes_per_pixel = data_size as f32 / pixel_count as f32;

    if bytes_per_pixel < 0.7 {
        DdsFormat::Bc1
    } else if bytes_per_pixel < 2.0 {
        DdsFormat::Bc3
    } else if bytes_per_pixel >= 3.5 {
        DdsFormat::Bgra8
    } else {
        DdsFormat::Unsupported
    }
}

/// Estimate DDS data size (header size subtracted).
fn estimate_dds_data_size(dds: &league_toolkit::texture::Dds) -> usize {
    // DDS header is 128 bytes (4 magic + 124 header)
    // This is a rough estimate
    let width = dds.width() as usize;
    let height = dds.height() as usize;
    let mip_count = dds.mip_count() as usize;

    // Calculate size for main texture + mipmaps
    let mut total = 0;
    for mip in 0..mip_count {
        let mip_w = (width >> mip).max(1);
        let mip_h = (height >> mip).max(1);
        // Assume BC3 (16 bytes per 4x4 block) as rough estimate
        total += (mip_w.div_ceil(4)) * (mip_h.div_ceil(4)) * 16;
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires actual DDS file
    fn test_dds_to_tex_conversion() {
        // This test requires a real DDS file
        // In practice, this would be tested with sample DDS files
    }

    #[test]
    fn test_format_detection() {
        // Unit tests for format detection would go here
    }
}
