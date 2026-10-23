//! A pack's source list (`apps/harness/packs/<name>.toml`, kept in the repository): which
//! pictures go into the pack and why, never the pictures. The pack's id, title and kind
//! (`shape` or `composition`), then per item:
//!
//! - `id`: the item's id in the pack, kept across editions (exercises name it);
//! - `tier`: one its kind takes (`entry`, `simple`, `medium`, `hard`; a composition no `entry`);
//! - `source`: `cleveland` (the Cleveland Museum of Art's Open Access API), `met` (the
//!   Metropolitan Museum of Art's Collection API), `aic` (the Art Institute of Chicago's API),
//!   `nga` (the National Gallery of Art's open data), `rijks` (the Rijksmuseum's Linked Art
//!   API) or `commons` (Wikimedia Commons' API);
//! - `object`: what the source knows it by: Cleveland's accession number, the Met's, the Art
//!   Institute's or the National Gallery's object id, the Rijksmuseum's object number, a Commons
//!   file's title (`File:…`);
//! - `at` (a Commons video only): the second of the frame taken, as a still;
//! - `licence`: the licence the API must state (`CC0`, `PD-US-gov`, `PD-old`, `CC BY 4.0`, …);
//! - `reason`: why it is in its tier, which the pack keeps as the item's notes;
//! - `no_outline` (a shape only, where it applies): why the item has no outline, its derived
//!   outline looked at on the contact sheet and judged wrong, or refused by the derivation itself
//!   (the build fails on a refusal the list does not record). A composition never has one.

use std::collections::HashSet;

use openpaint_practice::{Licence, PictureKind, Tier, valid_id};
use serde::Deserialize;

use crate::error::HarnessError;

/// Where an item's picture comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Cleveland,
    Met,
    Aic,
    Nga,
    Rijks,
    Commons,
}

impl Source {
    pub const ALL: [Self; 6] = [
        Self::Cleveland,
        Self::Met,
        Self::Aic,
        Self::Nga,
        Self::Rijks,
        Self::Commons,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Cleveland => "cleveland",
            Self::Met => "met",
            Self::Aic => "aic",
            Self::Nga => "nga",
            Self::Rijks => "rijks",
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
    #[serde(default)]
    at: Option<f64>,
    licence: String,
    reason: String,
    #[serde(default)]
    no_outline: Option<String>,
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
    /// The second of a Commons video's frame taken as the picture.
    pub at: Option<f64>,
    pub licence: Licence,
    pub reason: String,
    /// Why it has no outline, when it has none.
    pub no_outline: Option<String>,
}

impl Listed {
    /// What names its downloads in the cache: its object, and the second of its frame.
    pub fn key(&self) -> String {
        match self.at {
            Some(at) => format!("{}@{at}", self.object),
            None => self.object.clone(),
        }
    }
}

impl SourceList {
    /// The list in `text` (TOML).
    ///
    /// # Errors
    /// `List` for TOML that is no source list, a kind, tier, source or licence of no known name,
    /// a tier the kind does not take, an id that is not a pack's, taken twice, an item without
    /// its object or reason, a frame's second outside Commons or below zero, or `no_outline` on
    /// a kind never outlined.
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
            if !valid_id(&item.id) || !seen.insert(item.id.clone()) {
                let id = &item.id;
                return Err(HarnessError::List(format!(
                    "item {id}: its id is no item id or is taken"
                )));
            }
            items.push(listed(kind, item)?);
        }
        Ok(Self {
            id: written.id,
            title: written.title,
            kind,
            items,
        })
    }
}

/// One written item of a list of `kind`, checked.
fn listed(kind: PictureKind, item: WrittenItem) -> Result<Listed, HarnessError> {
    let wrong = |what: &str| HarnessError::List(format!("item {}: {what}", item.id));
    if item.object.trim().is_empty() || item.reason.trim().is_empty() {
        return Err(wrong("it has no object or no reason"));
    }
    match &item.no_outline {
        Some(_) if !kind.outlined() => return Err(wrong("its kind never has an outline")),
        Some(why) if why.trim().is_empty() => {
            return Err(wrong("it has no outline and says not why"));
        }
        _ => {}
    }
    let tier = Tier::of_name(&item.tier).ok_or_else(|| wrong("no such tier"))?;
    if !kind.tiers().contains(&tier) {
        return Err(wrong("its tier is not one its kind takes"));
    }
    let source = Source::of_name(&item.source).ok_or_else(|| wrong("no such source"))?;
    match item.at {
        Some(_) if source != Source::Commons => return Err(wrong("a frame comes from Commons")),
        Some(at) if !(at >= 0.0 && at.is_finite()) => return Err(wrong("its frame's second")),
        _ => {}
    }
    Ok(Listed {
        tier,
        source,
        licence: Licence::of_name(&item.licence).ok_or_else(|| wrong("no licence a pack takes"))?,
        id: item.id,
        object: item.object,
        at: item.at,
        reason: item.reason,
        no_outline: item.no_outline,
    })
}
