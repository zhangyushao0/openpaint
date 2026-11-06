//! Practice packs: the pictures practice copies, each with where it came from and the licence it
//! is used under, and its silhouette outline where one was derived and judged right. The app
//! ships no third-party picture; an official pack is built from a curated list by the harness
//! (`cargo practice-pack`) and imported beside the library, which keeps its bytes
//! (`openpaint-store`). The user's own pictures go into the personal pack, `PERSONAL`, kept on
//! the device: no credit or licence (pictures they chose for themselves), never exported, and
//! never mixed with an official pack's (an official pack credits every item, the personal pack
//! none). This is a pack's manifest and its rules; an exercise names a picture by its pack and its
//! id (`ItemRef`).

use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

use crate::areas::Areas;
use crate::licence::Licence;
use crate::outline::Outline;
use crate::value_masses::ValueMasses;

/// The longest side a pack's picture may have, in pixels.
pub const MAX_PICTURE_SIDE: u32 = 2048;
/// The longest id of a pack or an item, in bytes.
pub const MAX_ID: usize = 64;
/// The personal pack's id: the user's own pictures, each at the tier the user chose for it.
pub const PERSONAL: &str = "personal";
/// The most points an item's outline holds.
pub const MAX_OUTLINE_POINTS: usize = 8192;

/// What a picture is for: the exercise kind that copies it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PictureKind {
    /// One object alone, copied by its outline (V大's 复制单体图形).
    Shape,
    /// A whole picture of strong composition and clear colour and value structure: divided by
    /// its main lines (画面分割), reduced to two or three values (Notan), copied whole in line,
    /// colour blocks or colour, analysed for its focal point and contrasts. Compared by the
    /// picture itself laid in place, never by an outline.
    Composition,
    /// A crop of a real picture holding a few areas of distinct colour (S101, V大's 色彩组合: a
    /// corner of an abstract painting or of a rusted old thing), copied by eye as a colour
    /// combination against its areas, which the builder found and the pack keeps.
    Crop,
}

impl PictureKind {
    pub const ALL: [Self; 3] = [Self::Shape, Self::Composition, Self::Crop];

    /// As stored; a code names one kind for good.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Shape => 1,
            Self::Composition => 2,
            Self::Crop => 3,
        }
    }

    #[must_use]
    pub fn of_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.code() == code)
    }

    /// As a source list writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Shape => "shape",
            Self::Composition => "composition",
            Self::Crop => "crop",
        }
    }

    #[must_use]
    pub fn of_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }

    /// The tiers its pictures come in. Compositions skip Entry: even the simplest is a few big
    /// masses, so their three tiers follow how the structure is read: Simple (two or three big
    /// masses and one focal point, a Notan in two values), Medium (four to six masses, three
    /// values, secondary dividing lines), Hard (many masses, figures in space, close values).
    /// Crops likewise by their areas (`crop::tier_of`): three or four, five, six or seven.
    #[must_use]
    pub const fn tiers(self) -> &'static [Tier] {
        match self {
            Self::Shape => &Tier::ALL,
            Self::Composition | Self::Crop => &[Tier::Simple, Tier::Medium, Tier::Hard],
        }
    }

    /// Whether its pictures may carry a silhouette outline: a shape's does, a composition's
    /// never (its comparison is the picture in place).
    #[must_use]
    pub const fn outlined(self) -> bool {
        matches!(self, Self::Shape)
    }

    /// Whether its pictures may carry value masses (S96): a composition's do, a shape's never.
    #[must_use]
    pub const fn massed(self) -> bool {
        matches!(self, Self::Composition)
    }

    /// Whether its pictures carry colour areas (S101): a crop's always, no other's.
    #[must_use]
    pub const fn cropped(self) -> bool {
        matches!(self, Self::Crop)
    }
}

/// How hard a picture is to copy, easiest first; what each holds depends on the kind
/// (`PictureKind::tiers`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// Shapes: cartoon characters and creatures, bold closed outlines. No composition.
    Entry,
    /// Shapes: objects with clean outlines on plain backgrounds. Compositions: two or three big
    /// masses and one focal point.
    Simple,
    /// Shapes: animals, sculpture and everyday things, softer and longer outlines.
    /// Compositions: four to six masses in three values.
    Medium,
    /// Shapes: heads, armour, machines. Compositions: many masses, figures in space, close
    /// values.
    Hard,
}

impl Tier {
    pub const ALL: [Self; 4] = [Self::Entry, Self::Simple, Self::Medium, Self::Hard];

    /// As stored, 1 to 4.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Entry => 1,
            Self::Simple => 2,
            Self::Medium => 3,
            Self::Hard => 4,
        }
    }

    #[must_use]
    pub fn of_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|tier| tier.code() == code)
    }

    /// As a source list writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Simple => "simple",
            Self::Medium => "medium",
            Self::Hard => "hard",
        }
    }

    #[must_use]
    pub fn of_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tier| tier.name() == name)
    }
}

/// One picture of a pack: what it is, where it came from, and its outline.
#[derive(Debug, Clone, PartialEq)]
pub struct PackItem {
    /// Unique in its pack and kept across the pack's editions: exercises name it.
    pub id: String,
    pub kind: PictureKind,
    pub tier: Tier,
    /// The picture's width and height in pixels, at most `MAX_PICTURE_SIDE` each.
    pub size: [u32; 2],
    pub title: String,
    /// Where it comes from and the licence it is used under; none for the user's own picture.
    pub credit: Option<Credit>,
    /// Its silhouette (in the picture's pixels); none where none was derived or it was judged
    /// wrong: Compare then shows the picture itself, faint.
    pub outline: Option<Outline>,
    /// Its value masses (a composition's, S96), made when the pack was built and judged on its
    /// sheet; none where they were judged not to read as a squinting eye sees them: the picture
    /// is never offered for value masses then.
    pub masses: Option<Arc<ValueMasses>>,
    /// Its colour areas (a crop's, S101), found when the pack was built and judged on its sheet;
    /// a crop always has them, no other picture does.
    pub areas: Option<Arc<Areas>>,
}

/// What an official item's licence and its curator say about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credit {
    /// The page the picture comes from (https).
    pub source: String,
    pub creator: String,
    pub licence: Licence,
    /// The credit to show with the picture, word for word: what an Attribution licence asks for
    /// (title, creator, source, licence, changes), a courtesy line for the others.
    pub attribution: String,
    /// The curator's notes: why the picture is in its tier.
    pub notes: String,
}

impl PackItem {
    /// The credit that must show small under the picture wherever it is shown: an Attribution
    /// licence's.
    #[must_use]
    pub fn shown_credit(&self) -> Option<&str> {
        let credit = self.credit.as_ref()?;
        (credit.licence.requires_attribution()).then_some(credit.attribution.as_str())
    }
}

/// A pack's manifest: its pictures' items, in the pack's order.
#[derive(Debug, Clone, PartialEq)]
pub struct Pack {
    /// Names the pack (and its file) for good: lower-case letters, digits and hyphens.
    pub id: String,
    pub title: String,
    pub items: Vec<PackItem>,
}

/// How an exercise names a picture: its pack and its item.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemRef {
    pub pack: String,
    pub item: String,
}

/// A rule of a pack broken, and by which item (none: the pack's own fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackProblem {
    pub item: Option<String>,
    pub rule: &'static str,
}

impl fmt::Display for PackProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.item {
            Some(item) => write!(f, "item {item}: {}", self.rule),
            None => write!(f, "the pack: {}", self.rule),
        }
    }
}

impl std::error::Error for PackProblem {}

/// Whether `id` may name a pack or an item: 1 to `MAX_ID` lower-case ASCII letters, digits and
/// hyphens, starting with a letter or digit (so it is also a file name everywhere).
#[must_use]
pub fn valid_id(id: &str) -> bool {
    let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
    (1..=MAX_ID).contains(&id.len()) && !id.starts_with('-') && id.chars().all(allowed)
}

impl Pack {
    /// Whether it is the personal pack, the user's own pictures.
    #[must_use]
    pub fn personal(&self) -> bool {
        self.id == PERSONAL
    }

    /// The item `id`.
    #[must_use]
    pub fn item(&self, id: &str) -> Option<&PackItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// The item an exercise names, if it is in this pack.
    #[must_use]
    pub fn find(&self, item: &ItemRef) -> Option<&PackItem> {
        (item.pack == self.id)
            .then(|| self.item(&item.item))
            .flatten()
    }

    /// The items of `kind` and `tier`, in the pack's order.
    pub fn of(&self, kind: PictureKind, tier: Tier) -> impl Iterator<Item = &PackItem> {
        (self.items.iter()).filter(move |item| item.kind == kind && item.tier == tier)
    }

    /// Checks every rule of a pack: a valid id and a title; at least one item; each item's id
    /// valid and unique, a tier its kind takes, its picture's sides from 1 to
    /// `MAX_PICTURE_SIDE`, a title, an outline (if any) only on a kind outlined and of closed
    /// contours inside the picture; in an official pack every item credited
    /// (its source an https page, a creator, and an attribution where its licence asks for
    /// one), in the personal pack none.
    ///
    /// # Errors
    /// The first rule broken.
    pub fn check(&self) -> Result<(), PackProblem> {
        let pack = |rule| PackProblem { item: None, rule };
        if !valid_id(&self.id) {
            return Err(pack("its id is not lower-case letters, digits and hyphens"));
        }
        if self.title.trim().is_empty() {
            return Err(pack("it has no title"));
        }
        if self.items.is_empty() {
            return Err(pack("it holds no item"));
        }
        let mut seen = HashSet::new();
        for item in &self.items {
            let broken = |rule| PackProblem {
                item: Some(item.id.clone()),
                rule,
            };
            if !valid_id(&item.id) {
                return Err(broken(
                    "its id is not lower-case letters, digits and hyphens",
                ));
            }
            if !seen.insert(item.id.as_str()) {
                return Err(broken("its id is taken by an item before it"));
            }
            item.check().map_err(broken)?;
            match (&item.credit, self.personal()) {
                (Some(_), true) => return Err(broken("a picture of your own is credited")),
                (None, false) => return Err(broken("an official item has no credit")),
                (Some(credit), false) => credit.check().map_err(broken)?,
                (None, true) => {}
            }
        }
        Ok(())
    }
}

impl PackItem {
    /// The rules of one item but its id's and its credit's.
    fn check(&self) -> Result<(), &'static str> {
        let side = 1..=MAX_PICTURE_SIDE;
        if !self.size.iter().all(|length| side.contains(length)) {
            return Err("its picture's sides are not 1 to 2048 pixels");
        }
        if self.title.trim().is_empty() {
            return Err("it has no title");
        }
        if !self.kind.tiers().contains(&self.tier) {
            return Err("its tier is not one its kind takes");
        }
        if let Some(masses) = &self.masses {
            if !self.kind.massed() {
                return Err("its kind never has value masses and it has them");
            }
            if !masses.valid() {
                return Err("its value masses are no map of values");
            }
        }
        if self.kind.cropped() != self.areas.is_some() {
            return Err("a crop has no colour areas, or another picture has them");
        }
        let Some(outline) = &self.outline else {
            return Ok(());
        };
        if !self.kind.outlined() {
            return Err("its kind is compared without an outline and it has one");
        }
        let [width, height] = self.size.map(f64::from);
        let inside = |p: &[f64; 2]| (0.0..=width).contains(&p[0]) && (0.0..=height).contains(&p[1]);
        let points: usize = outline.contours.iter().map(Vec::len).sum();
        let closed = outline.contours.iter().all(|contour| contour.len() >= 3);
        if outline.contours.is_empty() || !closed || points > MAX_OUTLINE_POINTS {
            return Err("its outline is not closed contours");
        }
        if !outline.contours.iter().flatten().all(inside) {
            return Err("its outline leaves its picture");
        }
        Ok(())
    }
}

impl Credit {
    fn check(&self) -> Result<(), &'static str> {
        if !self.source.starts_with("https://") {
            return Err("its source is not an https page");
        }
        if self.creator.trim().is_empty() {
            return Err("it has no creator");
        }
        if self.licence.requires_attribution() && self.attribution.trim().is_empty() {
            return Err("its licence asks for an attribution and it has none");
        }
        Ok(())
    }
}
