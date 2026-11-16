//! `practice-pack search <source> <query> [count]`: candidates for a source list, for the curator
//! to look at before choosing: a line each (its number, object, the licence the API states, title)
//! and a contact sheet of their previews under `artifacts/packs/search/`. The query is the API's
//! own: Cleveland's or the Met's search parameters (`q=teapot&type=Silver`; plain words are `q=`),
//! a Commons category (`Category:…`) or Commons search words (`filew:>3000` among them), the Art
//! Institute's search words over its public-domain paintings (S159), a Texture Ninja tag (`rust`,
//! S159), Openverse's search parameters or words (S159; `source=wordpress` for photographs large
//! enough to crop), always within the licences a pack takes. Answers and previews are cached like
//! the builder's downloads; the lines give each candidate's size where the API states it.

use std::path::PathBuf;

use openpaint_store::decode_image;
use serde_json::{Value, json};

use super::fetch::{Fetcher, cache_name, fnv, query_text};
use super::libraries::TEXTURE_NINJA_PREVIEWS;
use super::list::Source;
use super::sheet::{Picture, write_sheet};
use super::sources::{AIC_HEADERS, text};
use crate::error::PackError;
use crate::repo_root;

/// A candidate as the search found it.
struct Candidate {
    object: String,
    licence: String,
    title: String,
    preview: String,
    /// The headers the preview's request carries.
    headers: &'static [(&'static str, &'static str)],
}

/// Runs a search, prints its candidates and writes their contact sheet.
///
/// # Errors
/// `Fetch` when the API cannot be reached; `Io` when the sheet cannot be written.
pub fn run(fetcher: &Fetcher, source: Source, query: &str, count: u32) -> Result<(), PackError> {
    let candidates = match source {
        Source::Cleveland => cleveland(fetcher, query, count)?,
        Source::Met => met(fetcher, query, count)?,
        Source::Commons => commons(fetcher, query, count)?,
        Source::Aic => aic(fetcher, query, count)?,
        Source::TextureNinja => texture_ninja(fetcher, query, count)?,
        Source::Openverse => openverse(fetcher, query, count)?,
        Source::Nga | Source::Rijks | Source::AmbientCg | Source::PolyHaven => {
            return Err(PackError::Missing(
                "search takes cleveland, met, commons, aic, textureninja or openverse; the \
                 others' items are found on their sites and checked by the build",
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
        let fetched = fetcher.get_with(&name, &candidate.preview, candidate.headers);
        let (size, rgba) = fetched
            .map_err(|error| error.to_string())
            .and_then(|bytes| decode_image(&bytes).map_err(|error| error.to_string()))
            .unwrap_or_else(|error| {
                println!("       its preview fails ({error}): a red dot");
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
    (repo_root().join("artifacts").join("packs").join("search"))
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

fn answer(
    fetcher: &Fetcher,
    source: Source,
    (url, headers): (&str, &[(&str, &str)]),
) -> Result<Value, PackError> {
    let name = format!("search/{}-{:016x}.json", source.name(), fnv(url));
    fetcher.json_with(&name, url, headers)
}

fn cleveland(fetcher: &Fetcher, query: &str, count: u32) -> Result<Vec<Candidate>, PackError> {
    let url = format!(
        "https://openaccess-api.clevelandart.org/api/artworks/?cc0=1&has_image=1&limit={count}&{}",
        parameters(query)
    );
    let answer = answer(fetcher, Source::Cleveland, (&url, &[]))?;
    let found = answer["data"].as_array().into_iter().flatten();
    Ok(found
        .filter_map(|data| {
            Some(Candidate {
                object: text(data, "accession_number")?,
                licence: text(data, "share_license_status")?,
                title: text(data, "title")?,
                preview: text(&data["images"]["web"], "url")?,
                headers: &[],
            })
        })
        .collect())
}

fn met(fetcher: &Fetcher, query: &str, count: u32) -> Result<Vec<Candidate>, PackError> {
    let url = format!(
        "https://collectionapi.metmuseum.org/public/collection/v1.1/search?hasImages=true\
         &limit={count}&{}",
        parameters(query)
    );
    let answer = answer(fetcher, Source::Met, (&url, &[]))?;
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
                headers: &[],
            });
        }
    }
    Ok(candidates)
}

fn commons(fetcher: &Fetcher, query: &str, count: u32) -> Result<Vec<Candidate>, PackError> {
    let api = "https://commons.wikimedia.org/w/api.php?action=query&format=json&formatversion=2";
    let list = if query.starts_with("Category:") {
        let category = query_text(query);
        format!("{api}&list=categorymembers&cmtype=file&cmlimit={count}&cmtitle={category}")
    } else {
        let words = query_text(query);
        format!("{api}&list=search&srnamespace=6&srlimit={count}&srsearch={words}")
    };
    let found = answer(fetcher, Source::Commons, (&list, &[]))?;
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
            "{api}&prop=imageinfo&iiprop=url%7Csize%7Cextmetadata&iiurlwidth=330\
             &iiextmetadatafilter=LicenseShortName%7CLicense&titles={joined}"
        );
        let info = answer(fetcher, Source::Commons, (&url, &[]))?;
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
            if let Some(image) = image {
                let Some(preview) = text(image, "thumburl") else {
                    continue;
                };
                candidates.push(Candidate {
                    object: title.clone(),
                    licence: format!("{} | {}", meta("LicenseShortName"), meta("License")),
                    title: format!(
                        "{} ({} x {})",
                        title.trim_start_matches("File:"),
                        image["width"],
                        image["height"]
                    ),
                    preview,
                    headers: &[],
                });
            }
        }
    }
    Ok(candidates)
}

/// The Art Institute's public-domain paintings (`is_public_domain`, `artwork_type_id` 1) its
/// search ranks first for `words`, a hundred a page.
fn aic(fetcher: &Fetcher, words: &str, count: u32) -> Result<Vec<Candidate>, PackError> {
    let mut candidates = Vec::new();
    for page in 1..=count.div_ceil(100) {
        let params = json!({
            "q": words,
            "query": {"bool": {"must": [
                {"term": {"is_public_domain": true}},
                {"term": {"artwork_type_id": 1}},
            ]}},
            "fields": ["id", "title", "artist_title", "image_id", "is_public_domain", "thumbnail"],
            "limit": 100,
            "page": page,
        });
        let url = format!(
            "https://api.artic.edu/api/v1/artworks/search?params={}",
            query_text(&params.to_string())
        );
        let found = answer(fetcher, Source::Aic, (&url, &AIC_HEADERS))?;
        let iiif = text(&found["config"], "iiif_url").unwrap_or_default();
        for data in found["data"].as_array().into_iter().flatten() {
            let (Some(id), Some(image)) = (data["id"].as_u64(), text(data, "image_id")) else {
                continue;
            };
            let public = data["is_public_domain"].as_bool() == Some(true);
            let thumbnail = &data["thumbnail"];
            candidates.push(Candidate {
                object: id.to_string(),
                licence: if public { "CC0" } else { "not public domain" }.to_owned(),
                title: format!(
                    "{}, {} ({} x {})",
                    text(data, "artist_title").unwrap_or_default(),
                    text(data, "title").unwrap_or_default(),
                    thumbnail["width"],
                    thumbnail["height"]
                ),
                preview: format!("{iiif}/{image}/full/200,/0/default.jpg"),
                headers: &AIC_HEADERS,
            });
        }
    }
    candidates.truncate(count as usize);
    Ok(candidates)
}

/// Texture Ninja's photographs of the tag `tag`, CC0 by the library's own dedication.
fn texture_ninja(fetcher: &Fetcher, tag: &str, count: u32) -> Result<Vec<Candidate>, PackError> {
    let url = format!("https://texture.ninja/api/textures?tag={}", query_text(tag));
    let found = answer(fetcher, Source::TextureNinja, (&url, &[]))?;
    Ok((found.as_array().into_iter().flatten())
        .filter_map(|texture| {
            let key = text(texture, "key")?;
            let tags: Vec<&str> = (texture["tags"].as_array().into_iter().flatten())
                .filter_map(|tag| tag["tag"].as_str())
                .collect();
            Some(Candidate {
                preview: format!("{TEXTURE_NINJA_PREVIEWS}{key}"),
                object: key,
                licence: "CC0, the library's".to_owned(),
                title: tags.join(", "),
                headers: &[],
            })
        })
        .take(count as usize)
        .collect())
}

/// Openverse's large images for `query` under a licence a pack takes, twenty a page (its most
/// without a key); each previewed by its provider's file, never Openverse's own thumbnail, so the
/// day's few requests go to the search.
fn openverse(fetcher: &Fetcher, query: &str, count: u32) -> Result<Vec<Candidate>, PackError> {
    let mut candidates = Vec::new();
    for page in 1..=count.div_ceil(20) {
        let url = format!(
            "https://api.openverse.org/v1/images/?{}&license=cc0,by,by-sa&size=large\
             &page_size=20&page={page}",
            parameters(query)
        );
        let found = answer(fetcher, Source::Openverse, (&url, &[]))?;
        let results = found["results"].as_array().cloned().unwrap_or_default();
        for image in &results {
            let (Some(id), Some(preview)) = (text(image, "id"), text(image, "url")) else {
                continue;
            };
            candidates.push(Candidate {
                object: id,
                licence: format!(
                    "{} {}",
                    text(image, "license").unwrap_or_default(),
                    text(image, "license_version").unwrap_or_default()
                ),
                title: format!(
                    "{}: {} ({} x {})",
                    text(image, "provider").unwrap_or_default(),
                    text(image, "title").unwrap_or_default(),
                    image["width"],
                    image["height"]
                ),
                preview,
                headers: &[],
            });
        }
        if results.len() < 20 {
            break;
        }
    }
    candidates.truncate(count as usize);
    Ok(candidates)
}
