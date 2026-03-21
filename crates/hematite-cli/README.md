# hematite-cli

Command-line interface for the Hematite skin fixer (v2).

## Status

✅ **Core functionality implemented:**
- Argument parsing with clap v4
- Logging system with colored terminal output + JSON mode
- Config loading (embedded fallback)
- Provider initialization (TxtHashProvider, LtkWadProvider, LtkBinProvider)
- Pipeline orchestration using hematite-core
- Single .bin file processing
- Human-readable and JSON output modes

⚠️ **Limitations (v2 MVP):**
- **.bin writing not yet implemented** (LTK 0.2/0.4 has private fields preventing reconstruction)
- WAD and Fantome processing deferred until LTK write support
- Champion list loading not yet implemented (uses empty default)
- Config fetching from GitHub not yet implemented (uses embedded config)

## Usage

```bash
# Process a single BIN file (read-only, shows what would be fixed)
hematite-cli path/to/skin.bin --all --dry-run

# Apply specific fixes
hematite-cli path/to/skin.bin --healthbar --white-model

# JSON output for automation
hematite-cli path/to/skin.bin --all --json

# Process directory (batch mode)
hematite-cli path/to/skins_folder/ --all

# Verbose logging
hematite-cli path/to/skin.bin --all -v verbose
```

## Available Fix Flags

| Flag | Description |
|------|-------------|
| `--healthbar` | Fix missing HP bar (UnitHealthBarStyle) |
| `--white-model` | Fix white model texture references |
| `--black-icons` | Fix black icons (.dds → .tex) |
| `--particles` | Fix broken particle textures |
| `--remove-champion-bins` | Remove outdated champion data |
| `--remove-bnk` | Remove incompatible audio files |
| `--vfx-shape` | Fix VFX shape format (14.1+ migration) |
| `--all` / `-a` | Enable all fixes |

## Building

```bash
# Development build
cargo build --package hematite-cli

# Release build (optimized)
cargo build --release --package hematite-cli

# Run directly
cargo run --package hematite-cli -- --help
```

## Architecture

**Modules:**
- `args.rs` — Clap derive argument parsing
- `logging.rs` — Tracing subscriber + colored console output
- `process.rs` — File routing (BIN, WAD, Fantome)
- `main.rs` — Entry point, config loading, result output

**Dependencies:**
- `hematite-types` — Config schema, data types
- `hematite-core` — Fix engine (detect + transform)
- `hematite-ltk` — LTK adapter (read-only for now)

## Next Steps

To enable full functionality:

1. **Implement BIN writing** (blocked by LTK API)
   - Wait for LTK to expose builder APIs for types with private `meta` fields
   - Or contribute upstream changes to league-toolkit

2. **WAD processing**
   - Extract BIN files from WAD
   - Apply fixes to each BIN
   - Rebuild WAD with modified BINs

3. **Fantome processing**
   - Unzip .fantome archive
   - Process contained .wad.client files
   - Repack modified WAD into archive

4. **Config fetching**
   - Fetch fix_config.json from GitHub with cache
   - Load champion_list.json from GitHub with cache
   - Fallback chain: remote → cache → embedded

5. **Champion list integration**
   - Load ChampionList from JSON
   - Build CharacterRelations lookup tables
   - Pass to FixContext for champion-specific transforms

## Testing

```bash
# Run tests for the full workspace
cargo test --workspace

# Lint check
cargo clippy --workspace -- -D warnings -A clippy::needless_return
```
