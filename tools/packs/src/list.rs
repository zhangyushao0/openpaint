//! A pack's source list (`tools/packs/lists/<name>.toml`, kept in the repository): which
//! pictures go into the pack and why, never the pictures. The pack's id, title and kind
//! (`shape`, `composition` or `crop`: a picture only cropped, S101), then per item:
//!
//! - `id`: the item's id in the pack, kept across editions (exercises name it); its crops' ids are
//!   it and their number, `<id>-1`, `<id>-2`;
//! - `tier`: one its kind takes (`entry`, `simple`, `medium`, `hard`; a composition no `entry`);
//!   none in a list of crops, whose tiers are their areas' (`crop::tier_of`);
//! - `source`: `cleveland` (the Cleveland Museum of Art's Open Access API), `met` (the
//!   Metropolitan Museum of Art's Collection API), `aic` (the Art Institute of Chicago's API),
//!   `nga` (the National Gallery of Art's open data), `rijks` (the Rijksmuseum's Linked Art
//!   API), `commons` (Wikimedia Commons' API), `ambientcg` or `polyhaven` (the two CC0 texture
//!   libraries' APIs, S101c: a material's colour map, only ever cropped), `textureninja` (Texture
//!   Ninja's CC0 photographs, S159, only ever cropped), `openverse` (Openverse's API, S159);
//! - `object`: what the source knows it by: Cleveland's accession number, the Met's, the Art
//!   Institute's or the National Gallery's object id, the Rijksmuseum's object number, a Commons
//!   file's title (`File:…`), an ambientCG asset id (`PaintedMetal006`), a Poly Haven asset id
//!   (`rusty_metal_02`), a Texture Ninja file (`PaintFlaking_0327.jpg`), an Openverse image's id;
//! - `at` (a Commons video only): the second of the frame taken, as a still;
//! - `licence`: the licence the API must state (`CC0`, `PD-US-gov`, `PD-old`, `CC BY 4.0`, …);
//! - `reason`: why it is in its tier, which the pack keeps as the item's notes;
//! - `no_outline` (a shape only, where it applies): why the item has no outline, its derived
//!   outline looked at on the contact sheet and judged wrong, or refused by the derivation itself
//!   (the build fails on a refusal the list does not record). A composition never has one.
//! - `no_masses` (a composition only, where it applies, S96): why the item has no value masses,
//!   its answer looked at beside it on the contact sheet and judged not to read as a squinting eye
//!   sees the picture; it is never offered for value masses. A shape never has them.
//! - `no_crop.<n>` (a composition's or a crop list's, S101): why its crop `n` is left out, the
//!   crop looked at beside its areas on the crops' sheet and judged no colour combination (an
//!   object recognisable, areas that do not read); one line, `no_crop.2 = "a bolt's head"`.
//! - `trim` (where a scan shows its frame or the canvas's edge, S158p): the shares of the picture
//!   cut from its top, right, bottom and left before anything is made of it (`trim.rs`).

use std::collections::{BTreeMap, HashSet};

use openpaint_practice::{Licence, PictureKind, Places, Tier, valid_id};
use serde::Deserialize;

use crate::error::PackError;
use crate::trim::{self, Trim};

/// Where an item's picture comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Cleveland,
    Met,
    Aic,
    Nga,
    Rijks,
    Commons,
    AmbientCg,
    PolyHaven,
    TextureNinja,
    Openverse,
}

impl Source {
    pub const ALL: [Self; 10] = [
        Self::Cleveland,
        Self::Met,
        Self::Aic,
        Self::Nga,
        Self::Rijks,
        Self::Commons,
        Self::AmbientCg,
        Self::PolyHaven,
        Self::TextureNinja,
        Self::Openverse,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Cleveland => "cleveland",
            Self::Met => "met",
            Self::Aic => "aic",
            Self::Nga => "nga",
            Self::Rijks => "rijks",
            Self::Commons => "commons",
            Self::AmbientCg => "ambientcg",
            Self::PolyHaven => "polyhaven",
            Self::TextureNinja => "textureninja",
            Self::Openverse => "openverse",
        }
    }

    /// A texture library's (S101c, S159): its pictures only ever cropped.
    pub const fn library(self) -> bool {
        matches!(self, Self::AmbientCg | Self::PolyHaven | Self::TextureNinja)
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
    #[serde(default)]
    tier: Option<String>,
    source: String,
    object: String,
    #[serde(default)]
    at: Option<f64>,
    licence: String,
    reason: String,
    #[serde(default)]
    no_outline: Option<String>,
    #[serde(default)]
    no_masses: Option<String>,
    #[serde(default)]
    no_crop: BTreeMap<String, String>,
    #[serde(default)]
    trim: Option<Trim>,
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
    /// Its tier; none in a list of crops, whose tiers are their areas'.
    pub tier: Option<Tier>,
    pub source: Source,
    pub object: String,
    /// The second of a Commons video's frame taken as the picture.
    pub at: Option<f64>,
    pub licence: Licence,
    pub reason: String,
    /// Why it has no outline, when it has none.
    pub no_outline: Option<String>,
    /// Why it has no value masses, when it has none (a composition's, S96).
    pub no_masses: Option<String>,
    /// Its crops left out, by number (from 1, as the crops' sheet numbers them within the item),
    /// each with why (S101).
    pub no_crop: BTreeMap<usize, String>,
    /// The shares of its picture cut from each side (S158p).
    pub trim: Option<Trim>,
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
    pub fn parse(text: &str) -> Result<Self, PackError> {
        let written: Written =
            toml::from_str(text).map_err(|error| PackError::List(error.to_string()))?;
        let kind = PictureKind::of_name(&written.kind)
            .ok_or_else(|| PackError::List(format!("no kind named {}", written.kind)))?;
        if !valid_id(&written.id) {
            return Err(PackError::List(format!("{} is no pack id", written.id)));
        }
        let mut seen = HashSet::new();
        let mut items = Vec::with_capacity(written.items.len());
        for item in written.items {
            if !valid_id(&item.id) || !seen.insert(item.id.clone()) {
                let id = &item.id;
                return Err(PackError::List(format!(
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
fn listed(kind: PictureKind, item: WrittenItem) -> Result<Listed, PackError> {
    let wrong = |what: &str| PackError::List(format!("item {}: {what}", item.id));
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
    match &item.no_masses {
        Some(_) if !kind.massed() => return Err(wrong("its kind never has value masses")),
        Some(why) if why.trim().is_empty() => {
            return Err(wrong("it has no value masses and says not why"));
        }
        _ => {}
    }
    let tier = match (&item.tier, kind) {
        (None, PictureKind::Crop) => None,
        (Some(_), PictureKind::Crop) => return Err(wrong("a crop's tier is its areas'")),
        (None, _) => return Err(wrong("it has no tier")),
        (Some(name), _) => Some(Tier::of_name(name).ok_or_else(|| wrong("no such tier"))?),
    };
    if tier.is_some_and(|tier| !kind.tiers().contains(&tier)) {
        return Err(wrong("its tier is not one its kind takes"));
    }
    let no_crop = no_crop(kind, &item.no_crop).map_err(|what| wrong(&what))?;
    if item.trim.is_some_and(|trim| !trim::valid(trim)) {
        return Err(wrong(
            "its trim: each side a share from 0 to a fifth, one above 0",
        ));
    }
    let source = Source::of_name(&item.source).ok_or_else(|| wrong("no such source"))?;
    if source.library() && kind != PictureKind::Crop {
        return Err(wrong("a texture library's picture is only ever cropped"));
    }
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
        no_masses: item.no_masses,
        no_crop,
        trim: item.trim,
    })
}

/// How many crops a picture of `kind` gives at most (S101; S159: every crop keeping the rules
/// and overlapping no better one by more than `OVERLAP`, up to these), and where they are taken:
/// a composition's two, at its corners; a picture only cropped six, anywhere; a shape none.
pub fn cropping(kind: PictureKind) -> Option<(Places, usize)> {
    match kind {
        PictureKind::Composition => Some((Places::Corners, 2)),
        PictureKind::Crop => Some((Places::Anywhere, 6)),
        PictureKind::Shape => None,
    }
}

/// The crops `written` lists out, by number, each with why: only for a kind cropped, a number
/// it gives, a reason given.
fn no_crop(
    kind: PictureKind,
    written: &BTreeMap<String, String>,
) -> Result<BTreeMap<usize, String>, String> {
    let most = cropping(kind).map_or(0, |(_, count)| count);
    (written.iter())
        .map(|(number, why)| {
            let crop: usize = number.parse().map_err(|_| format!("no crop {number}"))?;
            if !(1..=most).contains(&crop) {
                return Err(format!("its picture gives no crop {crop}"));
            }
            if why.trim().is_empty() {
                return Err(format!("crop {crop} is left out and says not why"));
            }
            Ok((crop, why.clone()))
        })
        .collect()
}
