# Hematite

> A high-performance League of Legends custom skin fixer built with Rust

![Status](https://img.shields.io/badge/status-v2_scaffolding-yellow)
![Rust](https://img.shields.io/badge/rust-2021_edition-orange)
![CLI](https://img.shields.io/badge/interface-CLI-blue)

## What it does

Hematite is a **config-driven** skin fixer that:
- Analyzes League of Legends custom skins for common issues
- Fixes broken health bars, white models, black icons, missing shaders, and VFX issues
- Supports single file and batch processing with parallel processing
- Updates fix logic via remote JSON config (no recompilation needed)

## Workspace Architecture

4-crate workspace. The core engine **never imports league-toolkit** — when LTK breaks its API, only the adapter crate changes.

```
hematite-v2/
├── Cargo.toml                         # Workspace manifest
├── config/
│   ├── fix_config.json                # Fix rule definitions
│   └── champion_list.json             # Champion metadata + subchamp relationships
├── crates/
│   ├── hematite-types/                # Pure data types (no LTK dependency)
│   │   └── src/
│   │       ├── hash.rs                # TypeHash, FieldHash, PathHash, GameHash newtypes
│   │       ├── bin.rs                 # BinTree, BinObject, PropertyValue (our types)
│   │       ├── config.rs              # FixConfig, DetectionRule, TransformAction schema
│   │       ├── champion.rs            # ChampionList, CharacterRelations
│   │       ├── result.rs              # ProcessResult, AppliedFix
│   │       └── wad.rs                 # WadChunkInfo, WadModification
│   │
│   ├── hematite-core/                 # Fix engine (no LTK dependency)
│   │   └── src/
│   │       ├── traits.rs              # BinProvider, HashProvider, WadProvider
│   │       ├── pipeline.rs            # detect -> transform -> result orchestration
│   │       ├── context.rs             # FixContext runtime state
│   │       ├── walk.rs                # PropertyWalker (replaces 6 recursive walks)
│   │       ├── filter.rs              # ObjectFilter (replaces 15+ inline loops)
│   │       ├── factory.rs             # ValueFactory (JSON -> PropertyValue)
│   │       ├── strings.rs             # Extension replace, FNV-1a, path normalize
│   │       ├── fallback.rs            # Jaro-Winkler asset similarity matching
│   │       ├── detect/rules.rs        # Detection rule dispatch
│   │       └── transform/             # One file per transform action
│   │           ├── ensure_field.rs    # Healthbar fix
│   │           ├── rename_hash.rs     # White model fix
│   │           ├── replace_ext.rs     # Black icons / particle fix
│   │           ├── change_type.rs     # Field type conversion
│   │           ├── regex_ops.rs       # Regex-based transforms
│   │           ├── vfx_shape.rs       # VFX shape migration (14.1+)
│   │           └── remove.rs          # Remove from WAD
│   │
│   ├── hematite-ltk/                  # LTK adapter (ONLY crate importing league-toolkit)
│   │   └── src/
│   │       ├── bin_adapter.rs         # impl BinProvider via ltk_meta
│   │       ├── hash_adapter.rs        # impl HashProvider (txt files, lmdb later)
│   │       ├── wad_adapter.rs         # impl WadProvider via ltk_wad
│   │       └── convert.rs             # LTK types <-> Hematite types
│   │
│   └── hematite-cli/                  # CLI binary
│       └── src/
│           ├── main.rs                # Entry point
│           ├── args.rs                # clap argument definitions
│           ├── logging.rs             # tracing + colored output
│           └── process.rs             # File routing (fantome/wad/bin/directory)
├── .github/workflows/
│   ├── release.yml                    # Tag-triggered release (git-cliff + binary)
│   └── ci.yml                         # PR checks (fmt + clippy + test)
└── cliff.toml                         # Conventional commit changelog config
```

### Dependency graph

```
hematite-cli  -->  hematite-core  -->  hematite-types
                   hematite-ltk   -->  hematite-types, league-toolkit
```

`hematite-core` and `hematite-types` have zero league-toolkit imports.

## Supported Fixes

| Fix | Detection | Transform | CLI Flag |
|-----|-----------|-----------|----------|
| Missing HP bar | `MissingOrWrongField` | `EnsureField` | `--healthbar` |
| White model (TextureName) | `FieldHashExists` | `RenameHash` | `--white-model` |
| White model (SamplerName) | `FieldHashExists` | `RenameHash` | `--white-model` |
| Black/missing icons | `StringExtensionNotInWad` | `ReplaceStringExtension` | `--black-icons` |
| Broken particles | `RecursiveStringExtensionNotInWad` | `ReplaceStringExtension` | `--particles` |
| Outdated champion data | `EntryTypeExistsAny` | `RemoveFromWad` | `--remove-champion-bins` |
| Incompatible audio | `BnkVersionNotIn` | `RemoveFromWad` | `--remove-bnk` |
| VFX shape (14.1+) | `VfxShapeNeedsFix` | `VfxShapeFix` | `--vfx-shape` |

All fixes are defined in `config/fix_config.json`. New fixes can be added by editing JSON without changing Rust code.

## Usage

```bash
# Auto-detect and fix all issues
hematite-cli "path/to/skin.fantome"

# Apply all available fixes
hematite-cli "skin.fantome" --all

# Fix specific issues
hematite-cli "skin.fantome" --healthbar --vfx-shape

# Dry run (show what would be fixed)
hematite-cli "skin.fantome" --dry-run

# JSON output for automation
hematite-cli "skin.fantome" --json > results.json

# Process entire directory
hematite-cli "path/to/skins_folder/"

# Verbose output
hematite-cli "skin.fantome" -v verbose

# View all options
hematite-cli --help
```

## Development

### Building

```bash
cd hematite-v2
cargo build --release --bin hematite-cli
# Binary: target/release/hematite-cli.exe
```

### Testing

```bash
cargo test --workspace
```

### Linting

```bash
cargo clippy --workspace -- -D warnings -A clippy::needless_return
cargo fmt --all -- --check
```

## Hash System

League uses hashes instead of strings:

| Hash kind | Width | Newtype | Source file |
|-----------|-------|---------|------------|
| Type hash | u32 | `TypeHash` | `hashes.bintypes.txt` |
| Field hash | u32 | `FieldHash` | `hashes.binfields.txt` |
| Path hash | u32 | `PathHash` | `hashes.binentries.txt` |
| Game hash | u64 | `GameHash` | `hashes.game.txt` |

Hash files stored at `%APPDATA%\RitoShark\Requirements\Hashes\`.

## Release Workflow

```bash
# 1. Bump version in workspace Cargo.toml
# 2. Commit and tag
git commit -m "chore: bump version to x.y.z"
git tag vx.y.z
# 3. Push — CI does the rest
git push && git push origin vx.y.z
```

CI generates changelog from conventional commits (git-cliff), builds Windows binary, and creates a GitHub release.

| Prefix | Changelog Section |
|--------|------------------|
| `feat:` | Features |
| `fix:` | Bug Fixes |
| `perf:` | Performance |
| `refactor:` | Refactor |
| `doc:` | Documentation |
| `chore:`, `ci:`, `build:` | Skipped |

## Why "Hematite"?

Hematite is the primary ore of iron. When iron oxidizes, it becomes *rust*. Since this tool is built in Rust and "cleans up" broken skins, the name is a fitting metaphor.

---

**Version:** 0.2.0
**Architecture:** 4-crate Rust workspace
