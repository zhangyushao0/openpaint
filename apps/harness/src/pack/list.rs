//! A pack's source list (`apps/harness/packs/<name>.toml`, kept in the repository): which
//! pictures go into the pack and why, never the pictures. The pack's id, title and kind, then
//! per item:
//!
//! - `id`: the item's id in the pack, kept across editions (exercises name it);
//! - `tier`: `entry`, `simple`, `medium` or `hard`;
//! - `source`: `cleveland` (the Cleveland Museum of Art's Open Access API), `met` (the
//!   Metropolitan Museum of Art's Collection API) or `commons` (Wikimedia Commons' API);
//! - `object`: what the source's API knows it by: Cleveland's accession number, the Met's object
//!   id, a Commons file's title (`File:…`);
//! - `licence`: the licence the API must state (`CC0`, `PD-US-gov`, `PD-old`, `CC BY 4.0`, …);
//! - `reason`: why it is in its tier, which the pack keeps as the item's notes.

use std::collections::HashSet;

use openpaint_practice::{Licence, PictureKind, Tier, valid_id};
use serde::Deserialize;

use crate::error::HarnessError;

/// Where an item's picture comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Cleveland,
    Met,
    Commons,
}

impl Source {
    pub const ALL: [Self; 3] = [Self::Cleveland, Self::Met, Self::Commons];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Cleveland => "cleveland",
            Self::Met => "met",
            Self::Commons => "commons",
        }
    }

    pub fn of_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|source| source.name() == name)
    }
}

/// A source list as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Written {
    id: String,
    title: String,
    kind: String,
    #[serde(rename = "item", default)]
    items: Vec<WrittenItem>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WrittenItem {
    id: String,
    tier: String,
    source: String,
    object: String,
    licence: String,
    reason: String,
}

/// A source list, read and checked.
#[derive(Debug, Clone)]
pub struct SourceList {
    pub id: String,
    pub title: String,
    pub kind: PictureKind,
    pub items: Vec<Listed>,
}

/// One item of a source list.
#[derive(Debug, Clone)]
pub struct Listed {
    pub id: String,
    pub tier: Tier,
    pub source: Source,
    pub object: String,
    pub licence: Licence,
    pub reason: String,
}

impl SourceList {
    /// The list in `text` (TOML).
    ///
    /// # Errors
    /// `List` for TOML that is no source list, a kind, tier, source or licence of no known name,
    /// an id that is not a pack's, taken twice, or an item without its object or reason.
    pub fn parse(text: &str) -> Result<Self, HarnessError> {
        let written: Written =
            toml::from_str(text).map_err(|error| HarnessError::List(error.to_string()))?;
        let kind = PictureKind::of_name(&written.kind)
            .ok_or_else(|| HarnessError::List(format!("no kind named {}", written.kind)))?;
        if !valid_id(&written.id) {
            return Err(HarnessError::List(format!("{} is no pack id", written.id)));
        }
        let mut seen = HashSet::new();
        let mut items = Vec::with_capacity(written.items.len());
        for item in written.items {
            let wrong = |what: &str| HarnessError::List(format!("item {}: {what}", item.id));
            if !valid_id(&item.id) || !seen.insert(item.id.clone()) {
                return Err(wrong("its id is no item id or is taken"));
            }
            if item.object.trim().is_empty() || item.reason.trim().is_empty() {
                return Err(wrong("it has no object or no reason"));
            }
            items.push(Listed {
                tier: Tier::of_name(&item.tier).ok_or_else(|| wrong("no such tier"))?,
                source: Source::of_name(&item.source).ok_or_else(|| wrong("no such source"))?,
                licence: Licence::of_name(&item.licence)
                    .ok_or_else(|| wrong("no licence a pack takes"))?,
                id: item.id,
                object: item.object,
                reason: item.reason,
            });
        }
        Ok(Self {
            id: written.id,
            title: written.title,
            kind,
            items,
        })
    }
}
