//! WAD file building — wraps ltk_wad's WadBuilder for use by the CLI.

use anyhow::Result;
use league_toolkit::wad::{WadBuilder, WadChunkBuilder};
use std::io::{Seek, Write};

/// Build a WAD file from an extracted file list, skipping removed paths.
///
/// `files` — all chunks as `(original_hash, resolved_path, bytes)`.
/// `files_to_remove` — paths to exclude from the output WAD.
/// `writer` — destination (file, cursor, etc.).
///
/// Returns the number of chunks written.
pub fn build_wad<W: Write + Seek>(
    files: &[(u64, String, Vec<u8>)],
    files_to_remove: &[String],
    writer: &mut W,
) -> Result<usize> {
    let mut builder = WadBuilder::default();
    let mut count = 0;

    for (hash, path, _) in files {
        if !files_to_remove.contains(path) {
            builder = builder.with_chunk(WadChunkBuilder::default().with_hash(*hash));
            count += 1;
        } else {
            tracing::debug!("Excluding removed file: {}", path);
        }
    }

    builder.build_to_writer(writer, |path_hash, cursor| {
        let (_, path, bytes) = files
            .iter()
            .find(|(h, _, _)| *h == path_hash)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Missing file for hash {:016X}", path_hash),
                )
            })?;

        tracing::trace!("Writing chunk: {} ({} bytes)", path, bytes.len());
        cursor.write_all(bytes)?;
        Ok(())
    })?;

    Ok(count)
}
