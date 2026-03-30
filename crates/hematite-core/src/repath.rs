//! Mod asset repathing.
//!
//! Inserts a short prefix after the first "/" of every asset path referenced
//! inside BIN files and renames the corresponding WAD entries to match.
//!
//! ## Why
//! League stores assets by xxhash64 of their lowercase path.  A mod that
//! ships `assets/characters/aatrox/skins/skin0/texture.dds` will silently
//! overwrite the base-game file with the same hash.  Repathing turns that
//! into `assets/bum/characters/aatrox/…`, giving it a distinct hash while
//! keeping the BIN → file relationship intact.
//!
//! ## Pipeline (called from `hematite-cli::process`)
//! 1. [`repath_bin_strings`] — walk every string value in each BIN tree and
//!    insert the prefix.  Returns a [`RepathBinResult`] with the count of
//!    changes and all new paths (for placeholder injection).
//! 2. [`repath_wad_path`] — compute the new WAD path for each non-BIN file.
//! 3. [`missing_invis_placeholders`] — (optional) build a list of invisible
//!    1×1 placeholder files for every texture path that has no matching WAD
//!    entry after repathing.

use crate::walk::{walk_tree, PropertyVisitor, VisitResult};
use hematite_types::bin::BinTree;
use hematite_types::hash::FieldHash;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Embedded placeholder texture
// ---------------------------------------------------------------------------

/// Bytes of an invisible 1×1 TEX texture used as a placeholder for missing assets.
pub const INVIS_TEX: &[u8] = include_bytes!("assets/invis.tex");

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Path prefixes that mark asset references we should repath.
const ASSET_PREFIXES: &[&str] = &["assets/", "data/"];

/// VO path fragment — never repath voice-over audio.
const VO_PATH: &str = "assets/sounds/wwise2016/vo/";

/// Texture extensions that trigger invisible placeholder injection.
/// Both `.dds` and `.tex` references are placeholded — always as `.tex`
/// since that is the native League format.
const TEXTURE_EXTS: &[&str] = &["dds", "tex"];

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Insert `prefix` after the first "/" in `path`.
///
/// ```
/// use hematite_core::repath::insert_prefix;
/// assert_eq!(
///     insert_prefix("assets/characters/aatrox/texture.dds", "bum"),
///     "assets/bum/characters/aatrox/texture.dds"
/// );
/// assert_eq!(insert_prefix("texture.dds", "bum"), "bum/texture.dds");
/// ```
pub fn insert_prefix(path: &str, prefix: &str) -> String {
    match path.find('/') {
        Some(pos) => format!("{}/{}/{}", &path[..pos], prefix, &path[pos + 1..]),
        None => format!("{}/{}", prefix, path),
    }
}

/// Returns `true` if this string value represents an asset path that should
/// be repathed.
fn is_repath_candidate(value: &str, skip_vo: bool) -> bool {
    let lower = value.to_lowercase();
    if !ASSET_PREFIXES.iter().any(|p| lower.starts_with(p)) {
        return false;
    }
    if skip_vo && lower.contains(VO_PATH) {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// BIN repathing
// ---------------------------------------------------------------------------

/// Result of repathing a single BIN tree.
pub struct RepathBinResult {
    /// Number of string values modified.
    pub strings_repathed: u32,
    /// All new (repathed) asset paths found in the tree.
    /// Used downstream to detect missing files for invis injection.
    pub new_paths: Vec<String>,
}

/// Repath all asset string references in a BIN tree and collect the new paths.
///
/// Modifies `tree` in-place.  Returns [`RepathBinResult`] with the change
/// count and the set of repathed paths (already lowercased for hash lookups).
pub fn repath_bin_strings(tree: &mut BinTree, prefix: &str, skip_vo: bool) -> RepathBinResult {
    struct RepathVisitor<'a> {
        prefix: &'a str,
        skip_vo: bool,
        new_paths: Vec<String>,
    }

    impl<'a> PropertyVisitor for RepathVisitor<'a> {
        fn visit_string(&mut self, value: &str, _field_hash: FieldHash) -> VisitResult {
            if is_repath_candidate(value, self.skip_vo) {
                let new_path = insert_prefix(value, self.prefix);
                self.new_paths.push(new_path.to_lowercase());
                VisitResult::Mutate(new_path)
            } else {
                VisitResult::Skip
            }
        }
    }

    let mut visitor = RepathVisitor {
        prefix,
        skip_vo,
        new_paths: Vec::new(),
    };

    let strings_repathed = walk_tree(tree, &mut visitor);

    RepathBinResult {
        strings_repathed,
        new_paths: visitor.new_paths,
    }
}

// ---------------------------------------------------------------------------
// WAD file repathing
// ---------------------------------------------------------------------------

/// Compute the repathed WAD path for a file entry.
///
/// BIN files are **not** repathed in the WAD — their content has already been
/// updated by [`repath_bin_strings`] and they must remain at their original
/// game-known paths so the engine can resolve them.
///
/// Returns `None` if the file should keep its original path.
pub fn repath_wad_path(path: &str, prefix: &str) -> Option<String> {
    let lower = path.to_lowercase();
    // BINs stay at original paths
    if lower.ends_with(".bin") {
        return None;
    }
    // Only repath known asset paths
    if !ASSET_PREFIXES.iter().any(|p| lower.starts_with(p)) {
        return None;
    }
    Some(insert_prefix(path, prefix))
}

// ---------------------------------------------------------------------------
// Invisible placeholder injection
// ---------------------------------------------------------------------------

/// Build a list of `(path, bytes)` placeholder entries for every texture path
/// that is referenced by BIN files but absent from the WAD after repathing.
///
/// Both `.dds` and `.tex` references are normalised to `.tex` (the native
/// League format) and filled with [`INVIS_TEX`] bytes.  Other extensions are
/// skipped entirely.
///
/// The returned paths are lowercased so the WAD builder hashes them correctly
/// with `xxhash64(path.to_lowercase())`.
pub fn missing_invis_placeholders(
    existing_paths: &HashSet<String>,
    referenced_paths: &[String],
) -> Vec<(String, Vec<u8>)> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    for path in referenced_paths {
        // paths are already lowercased from RepathBinResult
        let is_texture = TEXTURE_EXTS
            .iter()
            .any(|ext| path.ends_with(&format!(".{}", ext)));
        if !is_texture {
            continue;
        }

        // Normalise to .tex — League's native texture format
        let tex_path = if path.ends_with(".dds") {
            format!("{}.tex", &path[..path.len() - 4])
        } else {
            path.clone()
        };

        if !seen.insert(tex_path.clone()) {
            continue;
        }

        if existing_paths.contains(&tex_path) {
            continue;
        }

        result.push((tex_path, INVIS_TEX.to_vec()));
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_prefix_with_slash() {
        assert_eq!(
            insert_prefix("assets/characters/aatrox/skins/skin0/texture.dds", "bum"),
            "assets/bum/characters/aatrox/skins/skin0/texture.dds"
        );
    }

    #[test]
    fn test_insert_prefix_data_path() {
        assert_eq!(
            insert_prefix("data/characters/aatrox/skins/skin0.bin", "bum"),
            "data/bum/characters/aatrox/skins/skin0.bin"
        );
    }

    #[test]
    fn test_insert_prefix_no_slash() {
        assert_eq!(insert_prefix("texture.dds", "bum"), "bum/texture.dds");
    }

    #[test]
    fn test_repath_wad_path_skips_bin() {
        assert!(repath_wad_path("data/characters/aatrox/skins/skin0.bin", "bum").is_none());
    }

    #[test]
    fn test_repath_wad_path_renames_dds() {
        assert_eq!(
            repath_wad_path("assets/characters/aatrox/skins/skin0/texture.dds", "bum"),
            Some("assets/bum/characters/aatrox/skins/skin0/texture.dds".to_string())
        );
    }

    #[test]
    fn test_repath_wad_path_skips_unknown_prefix() {
        assert!(repath_wad_path("sounds/effects/explosion.bnk", "bum").is_none());
    }

    #[test]
    fn test_repath_bin_strings() {
        use hematite_types::bin::{BinObject, BinProperty, BinTree};
        use hematite_types::hash::{PathHash, TypeHash};
        use indexmap::IndexMap;
        use hematite_types::bin::PropertyValue;

        let mut tree = BinTree::default();
        let mut obj = BinObject {
            class_hash: TypeHash(0x1234),
            path_hash: PathHash(0x5678),
            properties: IndexMap::new(),
        };
        obj.properties.insert(
            0x1,
            BinProperty {
                name_hash: FieldHash(0x1),
                value: PropertyValue::String(
                    "assets/characters/aatrox/skins/skin0/texture.dds".to_string(),
                ),
            },
        );
        tree.objects.insert(0x5678, obj);

        let result = repath_bin_strings(&mut tree, "bum", true);
        assert_eq!(result.strings_repathed, 1);
        assert_eq!(result.new_paths.len(), 1);
        assert_eq!(
            result.new_paths[0],
            "assets/bum/characters/aatrox/skins/skin0/texture.dds"
        );
    }

    #[test]
    fn test_repath_bin_strings_skips_vo() {
        use hematite_types::bin::{BinObject, BinProperty, BinTree};
        use hematite_types::hash::{PathHash, TypeHash};
        use indexmap::IndexMap;
        use hematite_types::bin::PropertyValue;

        let mut tree = BinTree::default();
        let mut obj = BinObject {
            class_hash: TypeHash(0x1234),
            path_hash: PathHash(0x5678),
            properties: IndexMap::new(),
        };
        obj.properties.insert(
            0x1,
            BinProperty {
                name_hash: FieldHash(0x1),
                value: PropertyValue::String(
                    "assets/sounds/wwise2016/vo/aatrox/en_us/aatrox_base_vo.bnk".to_string(),
                ),
            },
        );
        tree.objects.insert(0x5678, obj);

        let result = repath_bin_strings(&mut tree, "bum", true);
        assert_eq!(result.strings_repathed, 0);
    }

    #[test]
    fn test_missing_invis_placeholders() {
        let existing: HashSet<String> = vec!["assets/bum/existing.tex".to_string()]
            .into_iter()
            .collect();
        let referenced = vec![
            "assets/bum/existing.dds".to_string(), // normalises to existing.tex — skipped
            "assets/bum/missing.dds".to_string(),  // normalises to missing.tex — injected
            "assets/bum/missing.tex".to_string(),  // already .tex — injected (deduped with above)
            "assets/bum/other.tex".to_string(),    // distinct .tex — injected
            "assets/bum/model.skn".to_string(),    // not a texture — skipped
        ];

        let placeholders = missing_invis_placeholders(&existing, &referenced);
        // missing.dds and missing.tex both normalise to missing.tex → 1 entry, not 2
        assert_eq!(placeholders.len(), 2);
        assert!(placeholders.iter().any(|(p, _)| p == "assets/bum/missing.tex"));
        assert!(placeholders.iter().any(|(p, _)| p == "assets/bum/other.tex"));
        // All placeholders must use INVIS_TEX bytes
        for (_, bytes) in &placeholders {
            assert_eq!(bytes.as_slice(), INVIS_TEX);
        }
    }
}
