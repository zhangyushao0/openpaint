//! A pack built from its source list: each item looked up through its source's API, the licence
//! the API states checked against the list's (a licence that disagrees fails the build and names
//! the item), its picture downloaded and made as the pack holds it (the store's `pack_picture`),
//! and, for a kind outlined (shapes), its silhouette outline derived unless the list says it has
//! none and why. The rule for every shape alike (`openpaint_practice::outline_of_matte`): a
//! picture on a clear ground keeps the outline of its own alpha, any other the outline of the
//! segmentation's matte (`segment.rs`). A composition is never outlined: the step does not run;
//! its value masses are made instead (S96, `ValueMasses::of`) and kept unless the list says they
//! do not read and why. Every download comes through the cache, and decoding, reducing, encoding,
//! segmenting and deriving are exact, so the same list and cache give the same pack byte for byte.

use std::sync::Arc;

use openpaint_practice::{
    Credit, Licence, Matte, Outline, OutlineRefusal, Pack, PackItem, PictureKind, ValueMasses,
    outline_of_matte,
};
use openpaint_store::{PackPicture, pack_picture};

use super::fetch::{Fetcher, cache_name};
use super::list::{Listed, SourceList};
use super::sources::{Found, Sources, Stated};
use crate::error::HarnessError;

/// What became of an item's outline.
#[derive(Debug, Clone, PartialEq)]
pub enum Outlined {
    /// Derived, and in the pack.
    Kept,
    /// The list says it has none, and why; what the derivation made of it, for the sheet.
    Listed {
        why: String,
        derived: Result<Outline, OutlineRefusal>,
    },
    /// The derivation refused it and the list does not say so: the pack is not written.
    Refused(OutlineRefusal),
    /// Its kind is compared without one (a composition): nothing was derived.
    Never,
}

/// What became of an item's value masses (S96).
#[derive(Debug, Clone, PartialEq)]
pub enum Massed {
    /// Made, and in the pack.
    Kept,
    /// The list says they do not read, and why; what was made, for the sheet.
    Listed { why: String, made: ValueMasses },
    /// Its kind has none (a shape).
    Never,
}

/// A pack as built: its manifest, its pictures' files in the list's order and what became of
/// each item's outline and value masses.
pub struct Built {
    pub pack: Pack,
    pub pictures: Vec<Vec<u8>>,
    pub outlines: Vec<Outlined>,
    pub masses: Vec<Massed>,
}

/// The segmentation's matte of a picture (8-bit sRGB RGBA of its size): the model's
/// (`Segmenter::matte`), or a test's own. Never called for a kind not outlined.
pub type Segment<'a> = &'a dyn Fn([u32; 2], &[u8]) -> Result<Matte, HarnessError>;

/// The pack `list` names and its pictures, in the list's order, outlined with `segment` when
/// its kind is.
///
/// # Errors
/// `Pack` naming the first item that cannot go in: its licence disagrees with the list's, its
/// source cannot be reached or its picture does not decode, or it breaks a rule of a pack.
pub fn build(
    list: &SourceList,
    fetcher: &Fetcher,
    segment: Segment<'_>,
) -> Result<Built, HarnessError> {
    let sources = Sources::new(fetcher);
    let mut items = Vec::with_capacity(list.items.len());
    let mut pictures = Vec::with_capacity(list.items.len());
    let mut outlines = Vec::with_capacity(list.items.len());
    let mut masses = Vec::with_capacity(list.items.len());
    for listed in &list.items {
        let (item, picture, (outlined, massed)) = item(list.kind, listed, (&sources, segment))
            .map_err(|error| match error {
                HarnessError::Pack { .. } => error,
                other => HarnessError::Pack {
                    item: listed.id.clone(),
                    problem: other.to_string(),
                },
            })?;
        items.push(item);
        pictures.push(picture);
        outlines.push(outlined);
        masses.push(massed);
    }
    let pack = Pack {
        id: list.id.clone(),
        title: list.title.clone(),
        items,
    };
    pack.check().map_err(|problem| HarnessError::Pack {
        item: problem.item.unwrap_or_else(|| list.id.clone()),
        problem: problem.rule.to_owned(),
    })?;
    Ok(Built {
        pack,
        pictures,
        outlines,
        masses,
    })
}

/// An item as built: its manifest entry, its picture's file, what became of its outline and of
/// its value masses.
type ItemBuilt = (PackItem, Vec<u8>, (Outlined, Massed));

fn item(
    kind: PictureKind,
    listed: &Listed,
    (sources, segment): (&Sources<'_>, Segment<'_>),
) -> Result<ItemBuilt, HarnessError> {
    let found = sources.look_up(listed)?;
    if found.stated != Stated::Licence(listed.licence) {
        return Err(HarnessError::Pack {
            item: listed.id.clone(),
            problem: format!(
                "its source states the licence {}, the list expects {}",
                found.stated.name(),
                listed.licence.name()
            ),
        });
    }
    let name = cache_name(listed.source.name(), &listed.key(), "img");
    let original = (sources.fetcher()).get_with(&name, &found.picture, found.headers)?;
    let picture = pack_picture(&original).map_err(|error| HarnessError::Fetch {
        url: found.picture.clone(),
        detail: format!("the picture: {error}"),
    })?;
    let (kept, outlined) = if kind.outlined() {
        outline(listed, &picture, segment)?
    } else {
        (None, Outlined::Never)
    };
    let (massed, masses) = if kind.massed() {
        masses(listed, &picture)
    } else {
        (None, Massed::Never)
    };
    let item = PackItem {
        id: listed.id.clone(),
        kind,
        tier: listed.tier,
        size: picture.size,
        title: found.title.clone(),
        credit: Some(Credit {
            attribution: attribution(&found, listed.licence),
            source: found.page,
            creator: found.creator,
            licence: listed.licence,
            notes: listed.reason.clone(),
        }),
        outline: kept,
        masses: massed,
    };
    Ok((item, picture.file, (outlined, masses)))
}

/// A shape's outline: derived from the segmentation's matte, kept unless the list says why not.
fn outline(
    listed: &Listed,
    picture: &PackPicture,
    segment: Segment<'_>,
) -> Result<(Option<Outline>, Outlined), HarnessError> {
    let matte = segment(picture.size, &picture.rgba)?;
    let derived = outline_of_matte(picture.size, &picture.rgba, &matte);
    Ok(match (&listed.no_outline, derived) {
        (Some(why), derived) => (
            None,
            Outlined::Listed {
                why: why.clone(),
                derived,
            },
        ),
        (None, Ok(outline)) => (Some(outline), Outlined::Kept),
        (None, Err(refusal)) => (None, Outlined::Refused(refusal)),
    })
}

/// A composition's value masses: made from its picture, kept unless the list says why not.
fn masses(listed: &Listed, picture: &PackPicture) -> (Option<Arc<ValueMasses>>, Massed) {
    let made = ValueMasses::of(picture.size, &picture.rgba);
    match &listed.no_masses {
        Some(why) => (
            None,
            Massed::Listed {
                why: why.clone(),
                made,
            },
        ),
        None => (Some(Arc::new(made)), Massed::Kept),
    }
}

/// The credit shown with the picture: for an Attribution licence what it asks for (title,
/// creator with the attribution the author asked for, source, licence and its deed, the change
/// made); for the others a courtesy line naming the collection.
pub fn attribution(found: &Found, licence: Licence) -> String {
    let Found {
        title,
        creator,
        page,
        credit,
        collection,
        changes,
        ..
    } = found;
    let by = if credit == creator {
        creator.clone()
    } else {
        format!("{creator} ({credit})")
    };
    match (licence, licence.deed()) {
        (licence, Some(deed)) if licence.requires_attribution() => format!(
            "“{title}” by {by}, {collection} ({page}), licensed {} ({deed}); {changes}.",
            licence.name()
        ),
        (_, Some(deed)) => format!("“{title}”, {creator}. {collection} ({page}), CC0 ({deed})."),
        (Licence::PdUsGov, None) => format!(
            "“{title}”, {creator}. {collection} ({page}); public domain, a work of the US \
             federal government."
        ),
        (_, None) => format!("“{title}”, {creator}. {collection} ({page}); public domain by age."),
    }
}
