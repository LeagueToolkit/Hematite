//! CLI argument definitions using clap derive.
//!
//! ## Available flags
//! | Flag | Fix |
//! |------|-----|
//! | `--healthbar` | Missing HP bar fix |
//! | `--white-model` | TextureName → TexturePath rename |
//! | `--black-icons` | .dds → .tex icon conversion |
//! | `--particles` | Broken particle texture fix |
//! | `--remove-champion-bins` | Remove outdated champion data |
//! | `--remove-bnk` | Remove incompatible audio files |
//! | `--vfx-shape` | VFX shape migration (14.1+) |
//! | `--all` / `-a` | Enable all fixes |
//!
//! ## Output control
//! | Flag | Effect |
//! |------|--------|
//! | `--json` | JSON output for automation |
//! | `--dry-run` | Show what would be fixed, don't modify |
//! | `-v <level>` | Verbosity: quiet, normal, verbose, trace |
//! | `-o <path>` | Output path (default: overwrite input) |

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "hematite-cli")]
#[command(about = "League of Legends custom skin fixer")]
#[command(version)]
pub struct Cli {
    /// Input file or directory to process
    pub input: PathBuf,

    /// Output path (default: overwrite input)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    // Fix flags
    #[arg(long, help = "Fix missing health bars")]
    pub healthbar: bool,

    #[arg(long, help = "Fix white models (TextureName → TexturePath)")]
    pub white_model: bool,

    #[arg(long, help = "Fix black/missing icons (.dds → .tex)")]
    pub black_icons: bool,

    #[arg(long, help = "Fix broken particle textures")]
    pub particles: bool,

    #[arg(long, help = "Remove outdated champion data BINs")]
    pub remove_champion_bins: bool,

    #[arg(long, help = "Remove incompatible BNK audio files")]
    pub remove_bnk: bool,

    #[arg(long, help = "Fix VFX shape format (14.1+ migration)")]
    pub vfx_shape: bool,

    #[arg(short, long, help = "Enable all fixes")]
    pub all: bool,

    // Output control
    #[arg(long, help = "JSON output for automation")]
    pub json: bool,

    #[arg(long, help = "Show what would be fixed without modifying files")]
    pub dry_run: bool,

    #[arg(short = 'v', long, default_value = "normal", help = "Verbosity level")]
    pub verbosity: Verbosity,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
    Trace,
}

/// All known fix IDs in application order.
const ALL_FIX_IDS: &[&str] = &[
    "healthbar_fix",
    "staticmat_texturepath",
    "staticmat_samplername",
    "black_icons",
    "dds_to_tex",
    "champion_bin_remover",
    "bnk_remover",
    "vfx_shape_fix",
];

/// Collect selected fix IDs based on CLI flags.
///
/// If `--all` is set or no flags are passed, returns all fix IDs.
/// Otherwise, returns only the specifically selected fixes.
pub fn collect_selected_fixes(cli: &Cli) -> Vec<String> {
    let mut fixes = Vec::new();
    if cli.healthbar { fixes.push("healthbar_fix".into()); }
    if cli.white_model {
        fixes.push("staticmat_texturepath".into());
        fixes.push("staticmat_samplername".into());
    }
    if cli.black_icons { fixes.push("black_icons".into()); }
    if cli.particles { fixes.push("dds_to_tex".into()); }
    if cli.remove_champion_bins { fixes.push("champion_bin_remover".into()); }
    if cli.remove_bnk { fixes.push("bnk_remover".into()); }
    if cli.vfx_shape { fixes.push("vfx_shape_fix".into()); }

    // If --all or no specific flags: apply all fixes
    if cli.all || fixes.is_empty() {
        return ALL_FIX_IDS.iter().map(|s| (*s).into()).collect();
    }

    fixes
}
