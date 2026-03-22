<p align="center">
  <h1 align="center">Hematite</h1>
  <p align="center">
    <strong>High-performance League of Legends custom skin fixer</strong>
  </p>
  <p align="center">
    <a href="https://github.com/LeagueToolkit/Hematite/releases"><img src="https://img.shields.io/github/v/release/LeagueToolkit/Hematite?style=flat-square&label=release&color=blue" alt="Release"></a>
    <img src="https://img.shields.io/badge/rust-2021_edition-orange?style=flat-square" alt="Rust">
    <img src="https://img.shields.io/badge/platform-windows-0078D6?style=flat-square" alt="Windows">
    <img src="https://img.shields.io/badge/tests-76_passing-brightgreen?style=flat-square" alt="Tests">
    <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License">
  </p>
</p>

---

Hematite automatically detects and fixes common issues in custom League of Legends skins. Drop in a `.fantome`, `.wad.client`, or `.bin` file and Hematite handles the rest — fixing broken health bars, invisible models, black icons, outdated audio, and more.

Fix rules are defined in JSON config, so new fixes can be added without recompiling.

## Features

- **Auto-detect mode** — runs all applicable fixes with zero configuration
- **Config-driven fixes** — add new fix rules via JSON, no code changes needed
- **Full write-back** — modified files are written back to disk (BIN, WAD, Fantome)
- **Batch processing** — process entire directories of skin files at once
- **LMDB hash system** — loads 1.8M game hashes in under 1 second
- **Remote config** — fetches latest fix rules from GitHub with offline fallback
- **Security hardened** — ZIP bomb protection, path traversal prevention, size limits
- **Dry-run mode** — preview what would be fixed before modifying files
- **JSON output** — machine-readable results for automation pipelines

## Quick Start

```bash
# Download latest release from GitHub Releases

# Fix a skin mod (auto-detects all issues)
hematite-cli "path/to/skin.fantome"

# Preview what would be fixed
hematite-cli "skin.fantome" --dry-run

# Fix specific issues only
hematite-cli "skin.fantome" --healthbar --vfx-shape

# Process a folder of mods
hematite-cli "path/to/skins_folder/"

# JSON output for scripts
hematite-cli "skin.fantome" --json > results.json
```

## Supported Fixes

| Fix | What it does | CLI Flag |
|-----|-------------|----------|
| **Health Bar** | Adds missing `UnitHealthBarStyle` field | `--healthbar` |
| **White Model** | Renames `TextureName`/`SamplerName` to correct hashes | `--white-model` |
| **Black Icons** | Converts `.dds` references to `.tex` | `--black-icons` |
| **Broken Particles** | Fixes particle texture paths recursively | `--particles` |
| **Champion Data** | Removes outdated champion BIN entries | `--remove-champion-bins` |
| **Audio Files** | Removes BNK files with incompatible Wwise versions | `--remove-bnk` |
| **VFX Shape** | Migrates VFX shape data to 14.1+ format | `--vfx-shape` |

Use `--all` or pass no flags to apply everything.

## Supported File Types

| Format | Description |
|--------|-------------|
| `.fantome` / `.zip` | Mod packages (extracts WAD, processes, rebuilds) |
| `.wad.client` | League asset archives (extracts BINs, fixes, rebuilds WAD) |
| `.bin` | League property files (parsed, fixed, written as `.fixed.bin`) |

## CLI Reference

```
hematite-cli [OPTIONS] <INPUT>

Arguments:
  <INPUT>    File or directory to process

Options:
  -o, --output <PATH>       Output path (default: creates .fixed.* next to input)
  -a, --all                 Enable all fixes
      --dry-run             Show what would be fixed without modifying files
      --json                JSON output for automation
  -v, --verbosity <LEVEL>   Verbosity: quiet, normal, verbose, trace [default: normal]
      --small-mod           Skip fallback assets (for texture-only mods)
      --all-skins           Process all skins found in mod

Fix flags:
      --healthbar             Fix missing health bars
      --white-model           Fix white models
      --black-icons           Fix black/missing icons
      --particles             Fix broken particle textures
      --remove-champion-bins  Remove outdated champion data
      --remove-bnk            Remove incompatible audio files
      --vfx-shape             Fix VFX shape format (14.1+)

  -h, --help     Print help
  -V, --version  Print version
```

## Architecture

4-crate Rust workspace. The core engine **never imports league-toolkit** — when LTK changes its API, only the adapter crate needs updating.

```
hematite-v2/
├── crates/
│   ├── hematite-types/   Pure data types, config schema, hash newtypes
│   ├── hematite-core/    Fix engine: detection, transforms, walker, fallback
│   ├── hematite-ltk/     LTK adapter: BIN parsing, WAD extraction, converters
│   └── hematite-cli/     CLI binary: args, logging, file routing, remote config
├── config/
│   ├── fix_config.json       Fix rule definitions
│   └── champion_list.json    Champion metadata + subchamp relationships
└── .github/workflows/
    ├── ci.yml                PR checks (fmt + clippy + test)
    └── release.yml           Tag-triggered release (git-cliff + binary)
```

### Dependency Graph

```
hematite-cli ──> hematite-core ──> hematite-types
                 hematite-ltk  ──> hematite-types + league-toolkit
```

### Processing Pipeline

```
Input file
  │
  ├─ .fantome/.zip ──> Extract .wad.client to temp dir
  │                          │
  ├─ .wad.client ────────────┤
  │                          ▼
  │                    Extract all files from WAD
  │                          │
  │                    ┌─────┴──────┐
  │                    │            │
  │               WAD pipeline   BIN pipeline
  │               (BNK removal,  (detect → transform
  │                DDS→TEX,       per BIN file)
  │                SCO→SCB)           │
  │                    │              │
  │                    └─────┬────────┘
  │                          │
  │                    Rebuild WAD with modifications
  │                          │
  │                    Write .fixed.wad.client
  │
  └─ .bin ──────────> Parse → detect → transform → write .fixed.bin
```

## Hash System

League uses hashed identifiers instead of strings. Hematite loads hash dictionaries at startup for name resolution.

| Hash Kind | Width | Source |
|-----------|-------|--------|
| Type hash | u32 | BIN class names (FNV-1a) |
| Field hash | u32 | BIN field names (FNV-1a) |
| Path hash | u32 | BIN entry paths (FNV-1a) |
| Game hash | u64 | WAD asset paths (xxhash64) |

**LMDB** (preferred): Single database file at `%APPDATA%\RitoShark\Requirements\Hashes\hashes.lmdb` — loads 1.8M hashes in ~800ms. Auto-downloaded from GitHub releases on first run.

**TXT fallback**: Individual text files in the same directory (`hashes.bintypes.txt`, `hashes.binfields.txt`, etc.)

## Building from Source

### Requirements

- Rust 1.75+ (2021 edition)
- Windows (primary target)

### Build

```bash
git clone https://github.com/LeagueToolkit/Hematite.git
cd Hematite
git checkout v2
cargo build --release --bin hematite-cli
# Binary: target/release/hematite-cli.exe
```

### Test

```bash
cargo test --workspace            # 76 tests
cargo clippy --workspace          # Lint check
cargo fmt --all -- --check        # Format check
```

## Release

Releases are automated via GitHub Actions. Push a version tag to trigger:

```bash
git tag v0.2.0
git push origin v0.2.0
# CI: generates changelog (git-cliff) → builds binary → creates GitHub Release
```

Commits follow [Conventional Commits](https://www.conventionalcommits.org/) for automatic changelog generation:

| Prefix | Changelog Section |
|--------|------------------|
| `feat:` | Features |
| `fix:` | Bug Fixes |
| `perf:` | Performance |
| `refactor:` | Refactor |

## Why "Hematite"?

Hematite is the primary ore of iron. When iron oxidizes, it becomes *rust*. This tool is built in Rust and cleans up broken skins — the name fits.

---

**Made by [RitoShark](https://github.com/RitoShark)** | Part of the [LeagueToolkit](https://github.com/LeagueToolkit) ecosystem
