//! Openverse (`api.openverse.org`, S159): Creative Commons' search across Flickr, the WordPress
//! Photo Directory, Wikimedia Commons and others, used without a key, as its terms allow (20
//! requests a minute and 200 a day; `fetch.rs` paces them). An image is read by its Openverse id
//! (`v1/images/<id>/`). Its licence is the answer's own `license` and `license_version`: `cc0`
//! 1.0, `by` and `by-sa` 4.0, 3.0, 2.5 and 2.0 are taken, any other refused; its page is the
//! provider's (`foreign_landing_url`), its creator the answer's `creator`. The picture is the
//! answer's `url`, but for a WordPress Photo Directory photograph, whose `url` is its 2,048-pixel
//! size: the original WordPress keeps beside its sizes (the size's `-<width>x<height>` taken off
//! the name), so its crops are cut from its full pixels. Flickr's `url` is at most 1,024 pixels
//! long, too small for a crop of `LEAST_LONGER`; a Commons file is listed from Commons itself.

use openpaint_practice::Licence;

use super::fetch::{Fetcher, cache_name, query_text};
use super::sources::{Found, Full, Stated, lacking, text, unknown_maker};
use crate::error::PackError;

/// The image `id`'s answer.
pub fn image_url(id: &str) -> String {
    format!("https://api.openverse.org/v1/images/{}/", query_text(id))
}

/// The Openverse image `id`, as its answer describes it.
///
/// # Errors
/// `Fetch` when the API cannot be reached, knows no such image, or its answer lacks a licence or a
/// picture.
pub fn found(fetcher: &Fetcher, id: &str) -> Result<Found, PackError> {
    let url = image_url(id);
    let answer = fetcher.json(&cache_name("openverse", id, "json"), &url)?;
    let licence = text(&answer, "license").ok_or_else(|| lacking(&url, "license"))?;
    let version = text(&answer, "license_version").unwrap_or_default();
    let picture = text(&answer, "url").ok_or_else(|| lacking(&url, "url"))?;
    let provider = text(&answer, "provider").unwrap_or_default();
    let creator = text(&answer, "creator").unwrap_or_else(|| unknown_maker(None));
    let title = text(&answer, "title").map_or_else(|| "Untitled".to_owned(), |t| first(&t));
    Ok(Found {
        title,
        credit: creator.clone(),
        creator,
        page: text(&answer, "foreign_landing_url")
            .ok_or_else(|| lacking(&url, "foreign_landing_url"))?,
        stated: stated(&licence, &version),
        picture: if provider == "wordpress" {
            original(&picture)
        } else {
            picture
        },
        collection: match provider.as_str() {
            "wordpress" => "WordPress Photo Directory",
            "flickr" => "Flickr",
            "wikimedia" => "Wikimedia Commons",
            _ => "Openverse",
        },
        changes: "resized",
        headers: &[],
        full: Full::Picture,
    })
}

/// The licence Openverse's `license` and `license_version` state.
pub fn stated(licence: &str, version: &str) -> Stated {
    let taken = match (licence, version) {
        ("cc0", "1.0") => Some(Licence::Cc0),
        ("by", "4.0") => Some(Licence::CcBy4),
        ("by", "3.0") => Some(Licence::CcBy3),
        ("by", "2.5") => Some(Licence::CcBy25),
        ("by", "2.0") => Some(Licence::CcBy2),
        ("by-sa", "4.0") => Some(Licence::CcBySa4),
        ("by-sa", "3.0") => Some(Licence::CcBySa3),
        ("by-sa", "2.5") => Some(Licence::CcBySa25),
        ("by-sa", "2.0") => Some(Licence::CcBySa2),
        _ => None,
    };
    taken.map_or_else(
        || Stated::Other(format!("{licence} {version}")),
        Stated::Licence,
    )
}

/// A WordPress file's original: its URL without the size WordPress gave its name
/// (`…/name-2048x1365.jpg` is `…/name.jpg`); the URL itself when it names no size.
pub fn original(url: &str) -> String {
    let Some((stem, extension)) = url.rsplit_once('.') else {
        return url.to_owned();
    };
    let sized = stem.rsplit_once('-').filter(|(_, size)| {
        let mut sides = size.split('x');
        let number = |side: Option<&str>| side.is_some_and(|s| s.parse::<u32>().is_ok());
        number(sides.next()) && number(sides.next()) && sides.next().is_none()
    });
    match sized {
        Some((name, _)) => format!("{name}.{extension}"),
        None => url.to_owned(),
    }
}

/// A title's first sentence (the WordPress Photo Directory's titles describe the photograph at
/// length), without its full stop.
fn first(title: &str) -> String {
    let sentence = title.split(". ").next().unwrap_or(title);
    sentence.trim().trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use openpaint_practice::Licence;

    use super::{first, original, stated};
    use crate::sources::Stated;

    #[test]
    fn a_licence_is_its_name_and_version_and_nothing_else_is_taken() {
        assert_eq!(stated("cc0", "1.0"), Stated::Licence(Licence::Cc0));
        assert_eq!(stated("by", "2.0"), Stated::Licence(Licence::CcBy2));
        assert_eq!(stated("by-sa", "4.0"), Stated::Licence(Licence::CcBySa4));
        for (licence, version) in [
            ("by-nc", "2.0"),
            ("by-nd", "4.0"),
            ("pdm", "1.0"),
            ("by", ""),
        ] {
            assert!(matches!(stated(licence, version), Stated::Other(_)));
        }
    }

    #[test]
    fn a_wordpress_size_leads_to_its_original() {
        assert_eq!(
            original("https://pd.w.org/2025/01/794677ae173eebd84.34072220-2048x1365.jpg"),
            "https://pd.w.org/2025/01/794677ae173eebd84.34072220.jpg"
        );
        let plain = "https://pd.w.org/2025/01/old-wall.jpg";
        assert_eq!(original(plain), plain);
        assert_eq!(
            first("A textured wall with peeling paint. The background is brown."),
            "A textured wall with peeling paint"
        );
    }
}
