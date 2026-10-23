//! What each source's API says about an object: its title, creator, page, the licence the API
//! states and the picture to download. The licence is read only from the API's own field
//! (Cleveland's `share_license_status`, the Met's `isPublicDomain`, Commons' machine-readable
//! `License` with its templates, `commons.rs`), never assumed from the source.

use openpaint_practice::Licence;
use serde_json::Value;

use super::commons;
use super::fetch::{Fetcher, cache_name, query_text};
use super::list::Source;
use crate::error::HarnessError;

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
    /// The picture to download: the largest the source hands out as a JPEG or PNG.
    pub picture: String,
    /// Who to credit: the creator, or the attribution the file's author asked for.
    pub credit: String,
    /// The collection the picture comes from.
    pub collection: &'static str,
}

/// `object` as `source`'s API describes it, its answer cached.
///
/// # Errors
/// `Fetch` when the API cannot be reached or does not answer as documented (a field missing).
pub fn look_up(fetcher: &Fetcher, source: Source, object: &str) -> Result<Found, HarnessError> {
    let name = cache_name(source.name(), object, "json");
    match source {
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
        Source::Commons => {
            let url = commons::info_url(object);
            commons::found(&fetcher.json(&name, &url)?, &url)
        }
    }
}

/// The text at `field`, trimmed; `None` when missing or empty.
pub fn text(value: &Value, field: &str) -> Option<String> {
    let text = value.get(field)?.as_str()?.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// The error for an answer of `url` lacking `field`.
pub fn lacking(url: &str, field: &str) -> HarnessError {
    HarnessError::Fetch {
        url: url.to_owned(),
        detail: format!("the answer has no {field}"),
    }
}

/// The Cleveland Museum of Art's Open Access API: `share_license_status` "CC0" for its open
/// access objects; the print picture (about 3,400 pixels on its long side).
fn cleveland(data: &Value, url: &str) -> Result<Found, HarnessError> {
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
        stated: if status == "CC0" {
            Stated::Licence(Licence::Cc0)
        } else {
            Stated::Other(status)
        },
        picture: text(&data["images"]["print"], "url")
            .ok_or_else(|| lacking(url, "print picture"))?,
        collection: "The Cleveland Museum of Art",
    })
}

/// The Metropolitan Museum of Art's Collection API: `isPublicDomain` true for its Open Access
/// objects, which the museum dedicates under CC0; the primary picture at its full size.
fn met(object: &Value, url: &str) -> Result<Found, HarnessError> {
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
        stated: if public {
            Stated::Licence(Licence::Cc0)
        } else {
            Stated::Other("not public domain".to_owned())
        },
        picture: text(object, "primaryImage").ok_or_else(|| lacking(url, "picture"))?,
        collection: "The Metropolitan Museum of Art",
    })
}

/// An anonymous object's creator, by its culture when the museum names one.
fn unknown_maker(culture: Option<&str>) -> String {
    match culture.map(str::trim).filter(|culture| !culture.is_empty()) {
        Some(culture) => format!("Unknown maker, {culture}"),
        None => "Unknown maker".to_owned(),
    }
}
