//! Entry validation transform — removes unreferenced entries.
//!
//! Removes ContextualActionData, AnimationGraphData, and GearSkinUpgrade entries
//! that aren't referenced by the main SkinCharacterDataProperties entry.
//! Stale entries from older patches can cause crashes or performance issues.

use crate::context::FixContext;
use crate::filter;
use hematite_types::bin::PropertyValue;
use hematite_types::config::EntryValidationTarget;

/// Remove entries not referenced by the main skin entry.
///
/// Returns number of entries removed.
pub fn apply(
    ctx: &mut FixContext<'_>,
    main_entry_type: &str,
    targets: &[EntryValidationTarget],
) -> u32 {
    let Some(main_type_hash) = ctx.hashes.type_hash(main_entry_type) else {
        return 0;
    };

    // Collect all referenced Link hashes from main entries
    let mut referenced_hashes = std::collections::HashSet::new();
    let mut found_main = false;
    for main_obj in filter::objects_by_type(&ctx.tree, main_type_hash) {
        found_main = true;
        collect_link_values(&main_obj.properties, &mut referenced_hashes);
    }

    if !found_main {
        return 0;
    }

    // Collect path_hashes of entries to remove
    let mut to_remove = Vec::new();

    for target in targets {
        let target_type_hash = if let Some(hex) = &target.type_hash {
            let hex = hex.trim_start_matches("0x");
            u32::from_str_radix(hex, 16).ok()
        } else {
            ctx.hashes.type_hash(&target.entry_type).map(|h| h.0)
        };

        let Some(type_hash) = target_type_hash else {
            continue;
        };

        for (&path_hash, obj) in &ctx.tree.objects {
            if obj.class_hash.0 == type_hash && !referenced_hashes.contains(&path_hash) {
                tracing::info!(
                    "Entry validator: removing unreferenced {} (path_hash: {:08X})",
                    target.entry_type,
                    path_hash
                );
                to_remove.push(path_hash);
            }
        }
    }

    // Remove collected entries
    let count = to_remove.len() as u32;
    for path_hash in &to_remove {
        ctx.tree.objects.swap_remove(path_hash);
    }

    if count > 0 {
        tracing::info!("Entry validator: removed {} unreferenced entries", count);
    }

    count
}

/// Recursively collect all Link hash values from a property map.
fn collect_link_values(
    properties: &indexmap::IndexMap<u32, hematite_types::bin::BinProperty>,
    out: &mut std::collections::HashSet<u32>,
) {
    for prop in properties.values() {
        collect_link_values_from_value(&prop.value, out);
    }
}

fn collect_link_values_from_value(value: &PropertyValue, out: &mut std::collections::HashSet<u32>) {
    match value {
        PropertyValue::Link(hash) => {
            if *hash != 0 {
                out.insert(*hash);
            }
        }
        PropertyValue::Struct(s) | PropertyValue::Embedded(s) => {
            collect_link_values(&s.properties, out);
        }
        PropertyValue::Container(items) | PropertyValue::UnorderedContainer(items) => {
            for item in items {
                collect_link_values_from_value(item, out);
            }
        }
        PropertyValue::Optional(boxed) => {
            if let Some(inner) = &**boxed {
                collect_link_values_from_value(inner, out);
            }
        }
        _ => {}
    }
}
