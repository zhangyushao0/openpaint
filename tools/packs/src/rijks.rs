//! The Rijksmuseum's Linked Art API (`data.rijksmuseum.nl`, no key). An object number is
//! searched (`search/collection?objectNumber=`), and each id found is read as Linked Art until
//! the one whose object number is the listed one. Its picture is the IIIF image its record names
//! as a digital object, and the licence that picture's own rights statement: Creative Commons'
//! Public Domain Mark (the museum's "Public Domain") as public domain by age, CC0 as CC0, any
//! other statement, none or more than one refused. The maker is the first person of its
//! production, read for their English name; the page is the object's persistent id.

use openpaint_practice::Licence;
use serde_json::Value;

use super::fetch::{Fetcher, cache_name, query_text};
use super::full::FULL_SIDE;
use super::sources::{Found, Full, Stated, lacking, text, unknown_maker};
use crate::error::PackError;

/// What a record is asked for as: Linked Art's JSON-LD.
const LINKED_ART: [(&str, &str); 1] = [("Accept", "application/ld+json")];
/// The Getty vocabulary's English, as a name's language.
const ENGLISH: &str = "http://vocab.getty.edu/aat/300388277";
const PUBLIC_DOMAIN_MARK: &str = "https://creativecommons.org/publicdomain/mark/1.0/";
const ZERO: &str = "https://creativecommons.org/publicdomain/zero/1.0/";

/// The object of object number `number`.
///
/// # Errors
/// `Fetch` when the API cannot be reached, finds no object of that number, or its record lacks
/// an English title or a IIIF picture.
pub fn found(fetcher: &Fetcher, number: &str) -> Result<Found, PackError> {
    let url = format!(
        "https://data.rijksmuseum.nl/search/collection?objectNumber={}",
        query_text(number)
    );
    let search = fetcher.json_with(&cache_name("rijks", number, "json"), &url, &LINKED_ART)?;
    let ids = (search["orderedItems"].as_array().into_iter().flatten())
        .filter_map(|item| text(item, "id"));
    for id in ids {
        let object = record(fetcher, &id)?;
        let numbered = (object["identified_by"].as_array().into_iter().flatten())
            .any(|name| name["type"] == "Identifier" && name["content"] == number);
        if numbered {
            return described(fetcher, &object, &id);
        }
    }
    Err(lacking(&url, "object of that number"))
}

/// The Linked Art record at `id`, cached.
fn record(fetcher: &Fetcher, id: &str) -> Result<Value, PackError> {
    fetcher.json_with(&cache_name("rijks", id, "json"), id, &LINKED_ART)
}

fn described(fetcher: &Fetcher, object: &Value, id: &str) -> Result<Found, PackError> {
    let (base, rights) = picture(object).ok_or_else(|| lacking(id, "IIIF picture"))?;
    let creator = match person(&object["produced_by"]) {
        Some(person) => english(&record(fetcher, &person)?["identified_by"])
            .unwrap_or_else(|| unknown_maker(None)),
        None => unknown_maker(None),
    };
    Ok(Found {
        title: english(&object["identified_by"]).ok_or_else(|| lacking(id, "English title"))?,
        credit: creator.clone(),
        creator,
        page: id.to_owned(),
        stated: stated(&rights),
        picture: format!("{base}/full/!2048,2048/0/default.jpg"),
        collection: "Rijksmuseum, Amsterdam",
        changes: "resized",
        headers: &[],
        full: Full::Iiif {
            base,
            most: FULL_SIDE,
            only_most: false,
        },
    })
}

/// The IIIF image service a record names (its base URL; its picture is fitted within 2,048
/// pixels) and that picture's statements.
fn picture(object: &Value) -> Option<(String, Vec<String>)> {
    let subjects = object["subject_of"].as_array()?;
    let carriers = (subjects.iter()).flat_map(|subject| {
        subject["digitally_carried_by"]
            .as_array()
            .into_iter()
            .flatten()
    });
    for carrier in carriers {
        let access = carrier["access_point"]
            .get(0)
            .and_then(|point| text(point, "id"));
        let Some(base) =
            access.and_then(|a| a.strip_suffix("/full/max/0/default.jpg").map(str::to_owned))
        else {
            continue;
        };
        let rights = (carrier["referred_to_by"].as_array().into_iter().flatten())
            .filter_map(|statement| text(statement, "content"))
            .collect();
        return Some((base, rights));
    }
    None
}

/// The licence a picture's statements give: exactly one, the Public Domain Mark or CC0.
pub fn stated(rights: &[String]) -> Stated {
    match rights {
        [one] if one == PUBLIC_DOMAIN_MARK => Stated::Licence(Licence::PdOld),
        [one] if one == ZERO => Stated::Licence(Licence::Cc0),
        [] => Stated::Other("no rights statement".to_owned()),
        many => Stated::Other(many.join(", ")),
    }
}

/// The first person's id anywhere in `value` (a record's production).
fn person(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            let id = map.get("id").and_then(Value::as_str);
            match (map.get("type").and_then(Value::as_str), id) {
                (Some("Person"), Some(id)) => Some(id.to_owned()),
                _ => map.values().find_map(person),
            }
        }
        Value::Array(items) => items.iter().find_map(person),
        _ => None,
    }
}

/// The first English name among a record's `identified_by`.
fn english(names: &Value) -> Option<String> {
    (names.as_array()?.iter())
        .filter(|name| name["type"] == "Name")
        .find(|name| {
            (name["language"].as_array().into_iter().flatten())
                .any(|language| language["id"] == ENGLISH)
        })
        .and_then(|name| text(name, "content"))
}
