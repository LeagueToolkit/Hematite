# Hematite v2 — Claude Code Rules

## Lint check
```bash
cargo clippy --workspace -- -D warnings -A clippy::needless_return
```

## Build
```bash
cd hematite-v2
cargo build --release --bin hematite-cli
```

## Test
```bash
cargo test --workspace
```

## Commits
- Never add `Co-Authored-By:` lines.
- Use conventional commits: `feat:`, `fix:`, `perf:`, `refactor:`, `doc:`, `test:`
- Commit messages should be short and imperative (e.g. `feat(core): add property walker`)
- Scopes: `types`, `core`, `ltk`, `cli`, `ci`

## Architecture

### Workspace — 4 crates
| Crate | Purpose | LTK? |
|-------|---------|------|
| `hematite-types` | Pure data types, config schema, hash newtypes | NO |
| `hematite-core` | Fix engine, detection, transforms, shared utilities | NO |
| `hematite-ltk` | LTK adapter (BIN/WAD/hash provider implementations) | YES |
| `hematite-cli` | CLI binary, logging, file processing, config fetching | NO |

### Key rule
`hematite-core` and `hematite-types` must NEVER import `league-toolkit`.
When LTK breaks, only `hematite-ltk` changes.

### Trait abstractions (in `hematite-core/src/traits.rs`)
- `BinProvider` — parse/write BIN files
- `HashProvider` — hash ↔ name resolution (txt files today, lmdb later)
- `WadProvider` — WAD path existence checks

### Shared utilities (in `hematite-core/src/`)
- `walk.rs` — PropertyWalker visitor pattern (replaces 6 recursive walks)
- `filter.rs` — ObjectFilter (replaces 15+ inline object loops)
- `factory.rs` — ValueFactory (JSON → PropertyValue conversion)
- `strings.rs` — Extension replace, FNV-1a hash, path normalization
- `fallback.rs` — Jaro-Winkler asset similarity matching

### Fix pipeline
```
Detection (detect/rules.rs) → Transform (transform/*.rs) → Result tracking
```
Each fix is defined in `config/fix_config.json` as a DetectionRule + TransformAction pair.

### Hash system
| Hash kind | Width | Newtype |
|-----------|-------|---------|
| Type hash (class_hash) | u32 | `TypeHash` |
| Field hash (name_hash) | u32 | `FieldHash` |
| Path hash (entry path) | u32 | `PathHash` |
| Game hash (WAD asset) | u64 | `GameHash` |

Hash files: `%APPDATA%\RitoShark\Requirements\Hashes\`

### Config loading
Fix config fetched from GitHub with fallback chain:
1. Remote fetch (1-hour cache TTL)
2. Local cache
3. Embedded default (`include_str!`)

## CI/CD
- `ci.yml` — fmt + clippy + test on PRs
- `release.yml` — tag-triggered (v*) → changelog + build + GitHub release
- `cliff.toml` — conventional commit parsing for changelog

## Release workflow
```bash
# 1. Bump version in workspace Cargo.toml
# 2. Commit and tag
git commit -m "chore: bump version to x.y.z"
git tag vx.y.z
git push && git push origin vx.y.z
```
