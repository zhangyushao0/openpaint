//! Wikimedia Commons' API (`api.php`: `imageinfo` with `extmetadata`, and the file page's
//! templates). The licence is the machine-readable `License` (`cc0`, `cc-by-4.0`, …, `pd`), and
//! a public-domain file names its reason by its licence template: `PD-USGov…` a US federal work,
//! `PD-old…` or `PD-Art…` by age. `LicenseShortName` must be there as well: a file whose page
//! states no licence is never taken. The picture is the original when it is a JPEG or PNG,
//! else the API's 1,280-pixel rendering (an SVG's, a TIFF's). A frame of a video (a Blender open
//! film's official release) is the API's 1,920-pixel still at the second the list names
//! (`seek`), under the video file's licence, titled with its time; for its crops the still is
//! asked again at the video's own width (`still_url`, S101b).

use openpaint_practice::Licence;
use serde_json::Value;

use super::fetch::query_text;
use super::sources::{Found, Full, Stated, lacking, text};
use crate::error::HarnessError;

/// The API's answer about the file `title` (`File:…`): its picture's URLs, size and type, its
/// metadata and its templates; with `at`, the still of a video at that second.
pub fn info_url(title: &str, at: Option<f64>) -> String {
    match at {
        Some(at) => still_url(title, at, 1920),
        None => answer_url(title, "iiurlwidth=1280"),
    }
}

/// The API's answer about the video `title` with its still at second `at`, `width` pixels wide.
pub fn still_url(title: &str, at: f64, width: u32) -> String {
    answer_url(
        title,
        &format!("iiurlwidth={width}&iiurlparam={width}px-seek%3D{at}"),
    )
}

/// The API's answer about `title`, its rendering asked as `rendering`.
fn answer_url(title: &str, rendering: &str) -> String {
    format!(
        "https://commons.wikimedia.org/w/api.php?action=query&format=json&formatversion=2\
         &prop=imageinfo%7Ctemplates&titles={}&iiprop=url%7Csize%7Cmime%7Cextmetadata\
         &{rendering}&tlnamespace=10&tllimit=max",
        query_text(title)
    )
}

/// The file in the API's `answer` (of `url`); with `at`, its still at that second.
///
/// # Errors
/// `Fetch` for a missing file, one whose page states no licence or no author, or a still asked
/// of a file that is no video.
pub fn found(answer: &Value, url: &str, at: Option<f64>) -> Result<Found, HarnessError> {
    let page = &answer["query"]["pages"][0];
    if page.get("missing").is_some() || page.get("invalid").is_some() {
        return Err(lacking(url, "file: Commons has no such file"));
    }
    let info = &page["imageinfo"][0];
    let meta = &info["extmetadata"];
    let short = meta_text(meta, "LicenseShortName").ok_or_else(|| lacking(url, "licence"))?;
    let code = meta_text(meta, "License").unwrap_or_default();
    let templates: Vec<&str> = (page["templates"].as_array().into_iter().flatten())
        .filter_map(|template| template["title"].as_str())
        .collect();
    let mime = text(info, "mime").unwrap_or_default();
    let picture = match at {
        Some(_) if !mime.starts_with("video/") => {
            return Err(lacking(url, "video to take a still of"));
        }
        None if mime == "image/jpeg" || mime == "image/png" => text(info, "url"),
        _ => text(info, "thumburl"),
    };
    let artist = meta_text(meta, "Artist").ok_or_else(|| lacking(url, "author"))?;
    // Commons' note on a file moved from another wiki is no part of the author's name.
    let creator = artist
        .split(" (transferred from ")
        .next()
        .unwrap_or_default()
        .to_owned();
    let file = text(page, "title").ok_or_else(|| lacking(url, "title"))?;
    let title = meta_text(meta, "ObjectName").unwrap_or_else(|| file_name(&file));
    Ok(Found {
        title: match at {
            Some(at) => format!("{title}, {}", minutes(at)),
            None => title,
        },
        credit: meta_text(meta, "Attribution").unwrap_or_else(|| creator.clone()),
        creator,
        page: text(info, "descriptionurl").ok_or_else(|| lacking(url, "page"))?,
        stated: stated(&code, &short, &templates),
        picture: picture.ok_or_else(|| lacking(url, "picture"))?,
        collection: "Wikimedia Commons",
        changes: if at.is_some() {
            "a still of the film, resized"
        } else {
            "resized"
        },
        headers: &[],
        full: match at {
            Some(_) => Full::Still {
                width: (info["width"].as_u64())
                    .and_then(|width| u32::try_from(width).ok())
                    .ok_or_else(|| lacking(url, "width"))?,
            },
            None => Full::Picture,
        },
    })
}

/// A second of a film as minutes and seconds, `2:05`.
fn minutes(at: f64) -> String {
    let whole = at.floor() as u64;
    format!("{}:{:02}", whole / 60, whole % 60)
}

/// The licence a file's machine-readable code and templates state.
pub fn stated(code: &str, short: &str, templates: &[&str]) -> Stated {
    let has = |prefix: &str| templates.iter().any(|t| t.starts_with(prefix));
    let licence = match code {
        "cc0" => Some(Licence::Cc0),
        "cc-by-4.0" => Some(Licence::CcBy4),
        "cc-by-3.0" => Some(Licence::CcBy3),
        "cc-by-2.5" => Some(Licence::CcBy25),
        "cc-by-2.0" => Some(Licence::CcBy2),
        "pd" if has("Template:PD-USGov") => Some(Licence::PdUsGov),
        "pd" if has("Template:PD-old") || has("Template:PD-Art") => Some(Licence::PdOld),
        _ => None,
    };
    licence.map_or_else(
        || Stated::Other(format!("{short} ({code})")),
        Stated::Licence,
    )
}

/// The text of an `extmetadata` field, its HTML taken away; `None` when missing or empty.
fn meta_text(meta: &Value, field: &str) -> Option<String> {
    let text = plain(meta[field]["value"].as_str()?);
    (!text.is_empty()).then_some(text)
}

/// A file's title without `File:` and its extension.
fn file_name(title: &str) -> String {
    let name = title.strip_prefix("File:").unwrap_or(title);
    name.rsplit_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_owned()
}

/// `html` as plain text: tags left out, the common entities read, runs of white space one space.
pub fn plain(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    let text = text
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
