//! Repath configuration types.

use std::path::PathBuf;

/// Options controlling the asset-repath pipeline.
///
/// Repathing inserts a prefix after the first "/" of every asset path
/// referenced inside BIN files (e.g. `assets/characters/…` →
/// `assets/bum/characters/…`), then renames the corresponding WAD
/// entries to match.  This prevents hash collisions with base-game files
/// and makes old mods work after a game update.
#[derive(Debug, Clone)]
pub struct RepathOptions {
    /// Prefix to insert into all asset paths.
    ///
    /// A short, unique string — e.g. `"bum"` → `assets/bum/…`.
    /// Defaults to `"bum"` if none is provided by the user.
    pub prefix: String,

    /// When `true`, inject an invisible 1×1 placeholder `.dds`/`.tex`
    /// for every texture path referenced in BIN files that has no
    /// corresponding file in the WAD.  Prevents black/missing-texture
    /// crashes without requiring the original assets.
    pub invis_texture: bool,

    /// Skip voice-over audio paths (`assets/sounds/wwise2016/vo/`).
    /// Defaults to `true` — VO files must always stay at their original
    /// paths so the game can find them.
    pub skip_vo: bool,

    /// Path to the base-game champion `.wad.client`.
    ///
    /// When set, files referenced by BIN strings but absent from the mod
    /// are extracted from this WAD and included in the output so the
    /// repathed mod is fully self-contained (like League-Mod-Repather).
    pub game_wad: Option<PathBuf>,
}

impl RepathOptions {
    /// Create options with sensible defaults.
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            invis_texture: false,
            skip_vo: true,
            game_wad: None,
        }
    }
}
