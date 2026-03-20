//! File processing orchestration.
//!
//! Routes input files to the appropriate processing pipeline based on file type:
//! - `.fantome` / `.zip` → extract ZIP, process WADs, repack
//! - `.wad.client` → process WAD directly
//! - `.bin` → process single BIN file
//! - Directory → walk and process all supported files
//!
//! ## Config loading
//! Fix config is loaded with a fallback chain:
//! 1. Fetch from GitHub (with 1-hour cache TTL)
//! 2. Fall back to local cache
//! 3. Fall back to embedded default
//!
//! ## Parallel processing
//! Uses rayon's par_iter for processing BIN chunks within a WAD file.
//!
//! ## TODO
//! - [ ] Implement process_file() routing
//! - [ ] Implement process_fantome() (ZIP extract → WAD process → repack)
//! - [ ] Implement process_wad() (mount → parallel BIN processing → rebuild)
//! - [ ] Implement process_bin() (single BIN file)
//! - [ ] Implement process_directory() (walkdir + parallel file processing)
//! - [ ] Implement config fetcher (GitHub → cache → embedded)
//! - [ ] Implement champion list loader (same fetch pattern)
