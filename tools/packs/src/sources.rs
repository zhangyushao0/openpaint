//! What each source's API says about an object: its title, creator, page, the licence the API
//! states and the picture to download. The licence is read only from the API's own field
//! (Cleveland's `share_license_status`, the Met's `isPublicDomain`, the Art Institute's
//! `is_public_domain`, the National Gallery's `openaccess`, `nga.rs`, the Rijksmuseum's rights
//! statement of its picture, `rijks.rs`, Commons' machine-readable `License` with its templates,
//! `commons.rs`), never assumed from the source; but for the two texture libraries, which
//! dedicate every asset to CC0 and state no licence per asset (`libraries.rs`, S101c).

use std::cell::OnceCell;

use openpaint_practice::Licence;
use serde_json::Value;

use super::fetch::{Fetcher, USER_AGENT, cache_name, query_text};
use super::list::{Listed, Source};
use super::{commons, libraries, nga, rijks};
use crate::error::PackError;

/// The licence an API states for an object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stated {
    Licence(Licence),
    /// No licence a pack takes: what the API said, word for word.
    Other(String),
}

impl Stated {
    pub fn name(&self) -> &str {
        match self {
            Self::Licence(licence) => licence.name(),
            Self::Other(text) => text,
        }
    }
}

/// An object as its source's API describes it.
#[derive(Debug, Clone)]
pub struct Found {
    pub title: String,
    pub creator: String,
    /// The object's page at the source.
    pub page: String,
    pub stated: Stated,
    /// The picture to download: the largest the source hands out as a JPEG or PNG (at most
    /// 2,048 pixels a side where the source sizes it).
    pub picture: String,
    /// Who to credit: the creator, or the attribution the file's author asked for.
    pub credit: String,
    /// The collection the picture comes from.
    pub collection: &'static str,
    /// What the pack's picture changes, as an Attribution licence's credit says it.
    pub changes: &'static str,
    /// The headers the source asks the picture's request to carry.
    pub headers: &'static [(&'static str, &'static str)],
    /// Where its full pixels come from, for its crops (S101b, `full.rs`).
    pub full: Full,
}

/// Where a picture's full pixels come from for its crops (S101b): never the packed picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Full {
    /// The picture downloaded is the source's own file at its full size: Commons' original (an
    /// SVG's or TIFF's rendering, the only one taken), the Met's primary picture, Cleveland's
    /// print, Poly Haven's colour map at 4K.
    Picture,
    /// A IIIF image service, by its base URL: the picture asked at its full size, its longer side
    /// at most `most` pixels (the most the server hands out); asked as `!most,most` alone when
    /// `only_most` (the Art Institute refuses any other fitted size and never enlarges).
    Iiif {
        base: String,
        most: u32,
        only_most: bool,
    },
    /// A film's still on Commons, asked again at the video's own width (pixels).
    Still { width: u32 },
    /// The colour map in the zip archive downloaded (an ambientCG material's, S101c), the map
    /// alone kept.
    Archive,
}

/// The sources' APIs through one cache, the National Gallery's tables read once for all items.
pub struct Sources<'a> {
    fetcher: &'a Fetcher,
    nga: OnceCell<nga::Tables>,
}

impl<'a> Sources<'a> {
    pub fn new(fetcher: &'a Fetcher) -> Self {
        Self {
            fetcher,
            nga: OnceCell::new(),
        }
    }

    pub fn fetcher(&self) -> &'a Fetcher {
        self.fetcher
    }

    /// The object `listed` names, as its source's API describes it, the answers cached.
    ///
    /// # Errors
    /// `Fetch` when the API cannot be reached or does not answer as documented (a field
    /// missing).
    pub fn look_up(&self, listed: &Listed) -> Result<Found, PackError> {
        let fetcher = self.fetcher;
        let object = listed.object.as_str();
        let name = cache_name(listed.source.name(), &listed.key(), "json");
        match listed.source {
            Source::Cleveland => {
                let url = format!(
                    "https://openaccess-api.clevelandart.org/api/artworks/{}",
                    query_text(object)
                );
                cleveland(&fetcher.json(&name, &url)?["data"], &url)
            }
            Source::Met => {
                let url = format!(
                    "https://collectionapi.metmuseum.org/public/collection/v1/objects/{}",
                    query_text(object)
                );
                met(&fetcher.json(&name, &url)?, &url)
            }
            Source::Aic => {
                let url = aic_url(object);
                aic(&fetcher.json_with(&name, &url, &AIC_HEADERS)?, &url)
            }
            Source::Nga => {
                let tables = match self.nga.get() {
                    Some(tables) => tables,
                    None => {
                        let loaded = nga::Tables::load(fetcher)?;
                        self.nga.get_or_init(|| loaded)
                    }
                };
                tables.found(object)
            }
            Source::Rijks => rijks::found(fetcher, object),
            Source::AmbientCg => libraries::ambientcg(fetcher, object),
            Source::PolyHaven => libraries::polyhaven(fetcher, object),
            Source::Commons => {
                let url = commons::info_url(object, listed.at);
                commons::found(&fetcher.json(&name, &url)?, &url, listed.at)
            }
        }
    }
}

/// The header the Art Institute asks every request to its API and pictures to carry: who is
/// asking (without it the requests are turned away).
pub const AIC_HEADERS: [(&str, &str); 1] = [("AIC-User-Agent", USER_AGENT)];
/// The longest side the Art Institute's IIIF server hands out (`!3000,3000`; it refuses larger
/// sizes, and most others, with 403, and gives a smaller picture at its own size, as tried on
/// 2026-11-06).
const AIC_MOST: u32 = 3000;

/// The text at `field`, trimmed; `None` when missing or empty.
pub fn text(value: &Value, field: &str) -> Option<String> {
    let text = value.get(field)?.as_str()?.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// The error for an answer of `url` lacking `field`.
pub fn lacking(url: &str, field: &str) -> PackError {
    PackError::Fetch {
        url: url.to_owned(),
        detail: format!("the answer has no {field}"),
    }
}

/// `Cc0` when `open`, else the API's own words for what it is.
fn cc0_when(open: bool, otherwise: &str) -> Stated {
    if open {
        Stated::Licence(Licence::Cc0)
    } else {
        Stated::Other(otherwise.to_owned())
    }
}

/// The Cleveland Museum of Art's Open Access API: `share_license_status` "CC0" for its open
/// access objects; the print picture (about 3,400 pixels on its long side).
fn cleveland(data: &Value, url: &str) -> Result<Found, PackError> {
    let status = text(data, "share_license_status").ok_or_else(|| lacking(url, "licence"))?;
    let culture = data["culture"].get(0).and_then(Value::as_str);
    let creator = data["creators"]
        .get(0)
        .and_then(|creator| text(creator, "description"))
        .unwrap_or_else(|| unknown_maker(culture));
    Ok(Found {
        title: text(data, "title").ok_or_else(|| lacking(url, "title"))?,
        credit: creator.clone(),
        creator,
        page: text(data, "url").ok_or_else(|| lacking(url, "page"))?,
        stated: cc0_when(status == "CC0", &status),
        picture: text(&data["images"]["print"], "url")
            .ok_or_else(|| lacking(url, "print picture"))?,
        collection: "The Cleveland Museum of Art",
        changes: "resized",
        headers: &[],
        full: Full::Picture,
    })
}

/// The Metropolitan Museum of Art's Collection API: `isPublicDomain` true for its Open Access
/// objects, which the museum dedicates under CC0; the primary picture at its full size.
fn met(object: &Value, url: &str) -> Result<Found, PackError> {
    let public = object["isPublicDomain"]
        .as_bool()
        .ok_or_else(|| lacking(url, "isPublicDomain"))?;
    let creator = text(object, "artistDisplayName")
        .unwrap_or_else(|| unknown_maker(object["culture"].as_str()));
    Ok(Found {
        title: text(object, "title").ok_or_else(|| lacking(url, "title"))?,
        credit: creator.clone(),
        creator,
        page: text(object, "objectURL").ok_or_else(|| lacking(url, "page"))?,
        stated: cc0_when(public, "not public domain"),
        picture: text(object, "primaryImage").ok_or_else(|| lacking(url, "picture"))?,
        collection: "The Metropolitan Museum of Art",
        changes: "resized",
        headers: &[],
        full: Full::Picture,
    })
}

/// The Art Institute of Chicago's API answer about artwork `id`, its fields only.
pub fn aic_url(id: &str) -> String {
    format!(
        "https://api.artic.edu/api/v1/artworks/{}?fields=id,title,artist_title,is_public_domain,\
         image_id",
        query_text(id)
    )
}

/// The Art Institute of Chicago's API: `is_public_domain` true for the works whose pictures the
/// museum releases under CC0; the picture from its IIIF server (the answer's `iiif_url`),
/// fitted within 2,048 pixels.
fn aic(answer: &Value, url: &str) -> Result<Found, PackError> {
    let data = &answer["data"];
    let public = data["is_public_domain"]
        .as_bool()
        .ok_or_else(|| lacking(url, "is_public_domain"))?;
    let id = data["id"].as_u64().ok_or_else(|| lacking(url, "id"))?;
    let image = text(data, "image_id").ok_or_else(|| lacking(url, "image_id"))?;
    let iiif = text(&answer["config"], "iiif_url").ok_or_else(|| lacking(url, "iiif_url"))?;
    let creator = text(data, "artist_title").unwrap_or_else(|| unknown_maker(None));
    Ok(Found {
        title: text(data, "title").ok_or_else(|| lacking(url, "title"))?,
        credit: creator.clone(),
        creator,
        page: format!("https://www.artic.edu/artworks/{id}"),
        stated: cc0_when(public, "not public domain"),
        picture: format!("{iiif}/{image}/full/!2048,2048/0/default.jpg"),
        collection: "The Art Institute of Chicago",
        changes: "resized",
        headers: &AIC_HEADERS,
        full: Full::Iiif {
            base: format!("{iiif}/{image}"),
            most: AIC_MOST,
            only_most: true,
        },
    })
}

/// An anonymous object's creator, by its culture when the museum names one.
pub fn unknown_maker(culture: Option<&str>) -> String {
    match culture.map(str::trim).filter(|culture| !culture.is_empty()) {
        Some(culture) => format!("Unknown maker, {culture}"),
        None => "Unknown maker".to_owned(),
    }
}
