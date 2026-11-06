//! `practice-pack search <cleveland|met|commons> <query> [count]`: candidates for a source list,
//! for the curator to look at before choosing: a line each (its number, object, the licence the
//! API states, title) and a contact sheet of their previews under `artifacts/packs/search/`.
//! The query is the API's own: Cleveland's or the Met's search parameters (`q=teapot&type=Silver`;
//! plain words are `q=`), a Commons category (`Category:…`) or Commons search words. Answers and
//! previews are cached like the builder's downloads.

use std::path::PathBuf;

use openpaint_store::decode_image;
use serde_json::Value;

use super::fetch::{Fetcher, cache_name, fnv, query_text};
use super::list::Source;
use super::sheet::{Picture, write_sheet};
use super::sources::text;
use crate::error::HarnessError;
use crate::output::workspace_root;

/// A candidate as the search found it.
struct Candidate {
    object: String,
    licence: String,
    title: String,
    preview: String,
}

/// Runs a search, prints its candidates and writes their contact sheet.
///
/// # Errors
/// `Fetch` when the API cannot be reached; `Io` when the sheet cannot be written.
pub fn run(fetcher: &Fetcher, source: Source, query: &str, count: u32) -> Result<(), HarnessError> {
    let candidates = match source {
        Source::Cleveland => cleveland(fetcher, query, count)?,
        Source::Met => met(fetcher, query, count)?,
        Source::Commons => commons(fetcher, query, count)?,
        Source::Aic | Source::Nga | Source::Rijks => {
            return Err(HarnessError::Missing(
                "search takes cleveland, met or commons; the others' items are found on their \
                 sites and checked by the build",
            ));
        }
    };
    let mut pictures = Vec::with_capacity(candidates.len());
    for (number, candidate) in candidates.iter().enumerate() {
        println!(
            "{:>3}  {}  {}  [{}]  {}",
            number + 1,
            source.name(),
            candidate.object,
            candidate.licence,
            candidate.title
        );
        let name = cache_name(
            &format!("preview-{}", source.name()),
            &candidate.object,
            "img",
        );
        let bytes = fetcher.get(&name, &candidate.preview)?;
        let (size, rgba) = decode_image(&bytes).unwrap_or_else(|error| {
            println!("       its preview does not decode ({error}): a red dot");
            ([1, 1], vec![255, 0, 0, 255])
        });
        pictures.push(Picture {
            size,
            rgba,
            outline: None,
            masses: None,
            areas: None,
        });
    }
    let sheet = sheet_path(source, query, count);
    write_sheet(&pictures, 1, &sheet)?;
    println!("{} candidates: {}", candidates.len(), sheet.display());
    Ok(())
}

fn sheet_path(source: Source, query: &str, count: u32) -> PathBuf {
    let key = fnv(&format!("{query}\n{count}")) as u32;
    (workspace_root()
        .join("artifacts")
        .join("packs")
        .join("search"))
    .join(format!("{}-{key:08x}.png", source.name()))
}

/// The API's parameters for `query`: as written when it holds some, else its words as `q`.
fn parameters(query: &str) -> String {
    if query.contains('=') {
        query.to_owned()
    } else {
        format!("q={}", query_text(query))
    }
}

fn answer(fetcher: &Fetcher, source: Source, url: &str) -> Result<Value, HarnessError> {
    let name = format!("search/{}-{:016x}.json", source.name(), fnv(url));
    fetcher.json(&name, url)
}

fn cleveland(fetcher: &Fetcher, query: &str, count: u32) -> Result<Vec<Candidate>, HarnessError> {
    let url = format!(
        "https://openaccess-api.clevelandart.org/api/artworks/?cc0=1&has_image=1&limit={count}&{}",
        parameters(query)
    );
    let answer = answer(fetcher, Source::Cleveland, &url)?;
    let found = answer["data"].as_array().into_iter().flatten();
    Ok(found
        .filter_map(|data| {
            Some(Candidate {
                object: text(data, "accession_number")?,
                licence: text(data, "share_license_status")?,
                title: text(data, "title")?,
                preview: text(&data["images"]["web"], "url")?,
            })
        })
        .collect())
}

fn met(fetcher: &Fetcher, query: &str, count: u32) -> Result<Vec<Candidate>, HarnessError> {
    let url = format!(
        "https://collectionapi.metmuseum.org/public/collection/v1.1/search?hasImages=true\
         &limit={count}&{}",
        parameters(query)
    );
    let answer = answer(fetcher, Source::Met, &url)?;
    let ids = answer["objectIDs"].as_array().into_iter().flatten();
    let mut candidates = Vec::new();
    for id in ids.filter_map(Value::as_u64).take(count as usize) {
        let object = id.to_string();
        let url = format!("https://collectionapi.metmuseum.org/public/collection/v1/objects/{id}");
        let Ok(found) = fetcher.json(&cache_name("met", &object, "json"), &url) else {
            println!("  met {object}: no answer, passed over");
            continue;
        };
        let public = found["isPublicDomain"].as_bool() == Some(true);
        if let (true, Some(preview)) = (public, text(&found, "primaryImageSmall")) {
            candidates.push(Candidate {
                object,
                licence: "CC0".to_owned(),
                title: text(&found, "title").unwrap_or_default(),
                preview,
            });
        }
    }
    Ok(candidates)
}

fn commons(fetcher: &Fetcher, query: &str, count: u32) -> Result<Vec<Candidate>, HarnessError> {
    let api = "https://commons.wikimedia.org/w/api.php?action=query&format=json&formatversion=2";
    let list = if query.starts_with("Category:") {
        let category = query_text(query);
        format!("{api}&list=categorymembers&cmtype=file&cmlimit={count}&cmtitle={category}")
    } else {
        let words = query_text(query);
        format!("{api}&list=search&srnamespace=6&srlimit={count}&srsearch={words}")
    };
    let found = answer(fetcher, Source::Commons, &list)?;
    let members = found["query"]["categorymembers"].as_array();
    let titles: Vec<String> = (members.or(found["query"]["search"].as_array()))
        .into_iter()
        .flatten()
        .filter_map(|member| text(member, "title"))
        .collect();
    let mut candidates = Vec::new();
    for batch in titles.chunks(50) {
        let joined = query_text(&batch.join("|"));
        let url = format!(
            "{api}&prop=imageinfo&iiprop=url%7Cextmetadata&iiurlwidth=330\
             &iiextmetadatafilter=LicenseShortName%7CLicense&titles={joined}"
        );
        let info = answer(fetcher, Source::Commons, &url)?;
        let pages = info["query"]["pages"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for title in batch {
            let page = pages
                .iter()
                .find(|page| text(page, "title").as_ref() == Some(title));
            let image = page.map(|page| &page["imageinfo"][0]);
            let meta = |field: &str| {
                let value = image.and_then(|image| image["extmetadata"][field]["value"].as_str());
                value.unwrap_or("none").to_owned()
            };
            if let Some(preview) = image.and_then(|image| text(image, "thumburl")) {
                candidates.push(Candidate {
                    object: title.clone(),
                    licence: format!("{} | {}", meta("LicenseShortName"), meta("License")),
                    title: title.trim_start_matches("File:").to_owned(),
                    preview,
                });
            }
        }
    }
    Ok(candidates)
}
