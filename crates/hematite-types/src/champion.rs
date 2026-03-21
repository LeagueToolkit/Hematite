//! Champion list and character-relation lookups.
//!
//! Loaded from `champion_list.json`. Provides:
//! - List of all champion names
//! - Subchamp mappings (e.g. Annie → Tibbers, Anivia → AniviaEgg)
//! - Healthbar values for non-champion entities (turrets, monsters, etc.)
//! - Reverse lookups: subchamp → primary champion
//!
//! ## Future
//! - Port extract_champion_from_path() from old character_relations.rs

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Raw champion list data from champion_list.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionList {
    pub version: String,
    pub champions: Vec<String>,
    pub subchamps: HashMap<String, Vec<String>>,
    pub healthbar_values: HashMap<String, u8>,
}

/// Pre-computed character relationship lookups.
///
/// Built from [`ChampionList`] at startup. All lookups are case-insensitive
/// (keys stored lowercase).
#[derive(Debug, Clone, Default)]
pub struct CharacterRelations {
    /// champion (lowercase) → list of subchamps
    pub champion_to_subchamps: HashMap<String, Vec<String>>,
    /// subchamp (lowercase) → primary champion
    pub subchamp_to_champion: HashMap<String, String>,
    /// entity name (lowercase) → healthbar value
    pub healthbar_values: HashMap<String, u8>,
}

impl CharacterRelations {
    /// Build from a raw champion list, pre-computing reverse maps.
    pub fn from_champion_list(list: &ChampionList) -> Self {
        let mut relations = Self::default();

        for (champion, subchamps) in &list.subchamps {
            let champion_lower = champion.to_lowercase();
            relations.champion_to_subchamps.insert(
                champion_lower.clone(),
                subchamps.clone(),
            );
            for sub in subchamps {
                relations.subchamp_to_champion.insert(
                    sub.to_lowercase(),
                    champion.clone(),
                );
            }
        }

        for (name, value) in &list.healthbar_values {
            relations.healthbar_values.insert(name.to_lowercase(), *value);
        }

        relations
    }

    /// Get related subchamps for a champion (case-insensitive).
    pub fn get_subchamps(&self, champion: &str) -> Option<&[String]> {
        self.champion_to_subchamps
            .get(&champion.to_lowercase())
            .map(|v| v.as_slice())
    }

    /// Get the primary champion for a subchamp (case-insensitive).
    pub fn get_primary_champion(&self, subchamp: &str) -> Option<&str> {
        self.subchamp_to_champion
            .get(&subchamp.to_lowercase())
            .map(|s| s.as_str())
    }
}
