//! Hash dictionary loading from LMDB database.
//!
//! Uses `heed` to read hash mappings from a single LMDB file with 4 named databases:
//! - "wad" → u64 game asset hashes (xxhash64)
//! - "types" → u32 BIN type hashes
//! - "fields" → u32 BIN field hashes
//! - "entries" → u32 BIN entry path hashes
//!
//! All hashes are loaded into memory at startup for O(1) lookups.

use anyhow::{Context, Result};
use heed::types::{Bytes, Str};
use heed::{Database, EnvOpenOptions};
use hematite_core::traits::HashProvider;
use hematite_types::hash::{FieldHash, GameHash, PathHash, TypeHash};
use std::collections::HashMap;
use std::path::PathBuf;

/// Hash provider backed by LMDB database.
///
/// Loads all hashes into memory at startup for O(1) lookups (similar to TxtHashProvider).
pub struct LmdbHashProvider {
    /// hash → type name
    types: HashMap<u32, String>,
    /// hash → field name
    fields: HashMap<u32, String>,
    /// hash → entry path
    entries: HashMap<u32, String>,
    /// hash → game asset path
    game_paths: HashMap<u64, String>,

    // Reverse maps (pre-computed at load time)
    /// type name (lowercase) → hash
    type_name_to_hash: HashMap<String, TypeHash>,
    /// field name (lowercase) → hash
    field_name_to_hash: HashMap<String, FieldHash>,
}

impl LmdbHashProvider {
    /// Get the RitoShark LMDB hash file path.
    pub fn get_hash_path() -> Result<PathBuf> {
        let appdata = std::env::var("APPDATA").context("APPDATA environment variable not set")?;
        Ok(PathBuf::from(appdata)
            .join("RitoShark")
            .join("Requirements")
            .join("Hashes")
            .join("hashes.lmdb"))
    }

    /// Load hash dictionaries from the standard RitoShark LMDB file.
    pub fn load_from_appdata() -> Result<Self> {
        let lmdb_path = Self::get_hash_path()?;

        if !lmdb_path.exists() {
            anyhow::bail!("LMDB hash file not found: {}", lmdb_path.display());
        }

        Self::load_from_path(&lmdb_path)
    }

    /// Load hash dictionaries from a specific LMDB file.
    pub fn load_from_path(lmdb_dir: &std::path::Path) -> Result<Self> {
        tracing::info!("Loading LMDB hashes from: {}", lmdb_dir.display());

        // Determine map_size from actual file size (LMDB defaults to 10 MB which is too small)
        // IMPORTANT: heed requires map_size to be a multiple of the OS page size
        let data_mdb = lmdb_dir.join("data.mdb");
        let page = page_size::get();
        let map_size = if data_mdb.exists() {
            let file_size = std::fs::metadata(&data_mdb)
                .map(|m| m.len() as usize)
                .unwrap_or(0);
            // Use file size + 25% headroom, minimum 100 MB, rounded up to page boundary
            let min_size = 100 * 1024 * 1024;
            let raw = std::cmp::max(file_size + file_size / 4, min_size);
            raw.div_ceil(page) * page
        } else {
            1024 * 1024 * 1024 // 1 GB fallback (already page-aligned)
        };

        // Open LMDB environment (read-only)
        let mut opts = EnvOpenOptions::new();
        opts.max_dbs(4);
        opts.map_size(map_size);
        let env = unsafe {
            opts.open(lmdb_dir)
                .context("Failed to open LMDB environment")?
        };

        let rtxn = env.read_txn().context("Failed to start read transaction")?;

        // Open each named database
        let wad_db: Database<Bytes, Str> = env
            .open_database(&rtxn, Some("wad"))
            .context("Failed to open 'wad' database")?
            .context("'wad' database not found")?;

        let types_db: Database<Bytes, Str> = env
            .open_database(&rtxn, Some("types"))
            .context("Failed to open 'types' database")?
            .context("'types' database not found")?;

        let fields_db: Database<Bytes, Str> = env
            .open_database(&rtxn, Some("fields"))
            .context("Failed to open 'fields' database")?
            .context("'fields' database not found")?;

        let entries_db: Database<Bytes, Str> = env
            .open_database(&rtxn, Some("entries"))
            .context("Failed to open 'entries' database")?
            .context("'entries' database not found")?;

        // Load all hashes into memory
        let mut types = HashMap::new();
        for item in types_db
            .iter(&rtxn)
            .context("Failed to iterate types database")?
        {
            let (key_bytes, name) = item.context("Failed to read type entry")?;
            if key_bytes.len() == 4 {
                let hash =
                    u32::from_be_bytes([key_bytes[0], key_bytes[1], key_bytes[2], key_bytes[3]]);
                types.insert(hash, name.to_string());
            }
        }

        let mut fields = HashMap::new();
        for item in fields_db
            .iter(&rtxn)
            .context("Failed to iterate fields database")?
        {
            let (key_bytes, name) = item.context("Failed to read field entry")?;
            if key_bytes.len() == 4 {
                let hash =
                    u32::from_be_bytes([key_bytes[0], key_bytes[1], key_bytes[2], key_bytes[3]]);
                fields.insert(hash, name.to_string());
            }
        }

        let mut entries = HashMap::new();
        for item in entries_db
            .iter(&rtxn)
            .context("Failed to iterate entries database")?
        {
            let (key_bytes, name) = item.context("Failed to read entry entry")?;
            if key_bytes.len() == 4 {
                let hash =
                    u32::from_be_bytes([key_bytes[0], key_bytes[1], key_bytes[2], key_bytes[3]]);
                entries.insert(hash, name.to_string());
            }
        }

        let mut game_paths = HashMap::new();
        for item in wad_db
            .iter(&rtxn)
            .context("Failed to iterate wad database")?
        {
            let (key_bytes, name) = item.context("Failed to read wad entry")?;
            if key_bytes.len() == 8 {
                let hash = u64::from_be_bytes([
                    key_bytes[0],
                    key_bytes[1],
                    key_bytes[2],
                    key_bytes[3],
                    key_bytes[4],
                    key_bytes[5],
                    key_bytes[6],
                    key_bytes[7],
                ]);
                game_paths.insert(hash, name.to_string());
            }
        }

        rtxn.commit().context("Failed to commit read transaction")?;

        // Build reverse lookups
        let type_name_to_hash = types
            .iter()
            .map(|(hash, name)| (name.to_lowercase(), TypeHash(*hash)))
            .collect();

        let field_name_to_hash = fields
            .iter()
            .map(|(hash, name)| (name.to_lowercase(), FieldHash(*hash)))
            .collect();

        tracing::info!(
            "Loaded LMDB hashes: {} game paths, {} types, {} fields, {} entries",
            game_paths.len(),
            types.len(),
            fields.len(),
            entries.len()
        );

        Ok(Self {
            types,
            fields,
            entries,
            game_paths,
            type_name_to_hash,
            field_name_to_hash,
        })
    }
}

impl HashProvider for LmdbHashProvider {
    fn resolve_type(&self, hash: TypeHash) -> Option<&str> {
        self.types.get(&hash.0).map(|s| s.as_str())
    }

    fn resolve_field(&self, hash: FieldHash) -> Option<&str> {
        self.fields.get(&hash.0).map(|s| s.as_str())
    }

    fn resolve_entry(&self, hash: PathHash) -> Option<&str> {
        self.entries.get(&hash.0).map(|s| s.as_str())
    }

    fn resolve_game_path(&self, hash: GameHash) -> Option<&str> {
        self.game_paths.get(&hash.0).map(|s| s.as_str())
    }

    fn type_hash(&self, name: &str) -> Option<TypeHash> {
        self.type_name_to_hash.get(&name.to_lowercase()).copied()
    }

    fn field_hash(&self, name: &str) -> Option<FieldHash> {
        self.field_name_to_hash.get(&name.to_lowercase()).copied()
    }

    fn has_game_path(&self, path: &str) -> bool {
        use xxhash_rust::xxh64::xxh64;

        let normalized = path.to_lowercase().replace('\\', "/");
        let hash = xxh64(normalized.as_bytes(), 0);
        self.game_paths.contains_key(&hash)
    }

    fn is_loaded(&self) -> bool {
        !self.types.is_empty() || !self.fields.is_empty()
    }
}
