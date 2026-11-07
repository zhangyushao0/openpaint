//! The National Gallery of Art, Washington, through its open data (CC0): no API answers about one
//! object, so its two daily tables are read whole, as exported at one pinned commit of
//! `NationalGalleryOfArt/opendata` (the export of 2026-10-23), and cached like any download.
//! `objects.csv` gives an object's title and attribution; `published_images.csv` its pictures,
//! the primary one's IIIF service and its `openaccess` field: 1 "open access image with no usage
//! restrictions under the NGA's open access policy" (which dedicates them CC0), 0 "rights
//! restricted image available only under fair use doctrine" (the data dictionary's words).

use std::collections::HashMap;

use openpaint_practice::Licence;

use super::fetch::Fetcher;
use super::full::FULL_SIDE;
use super::sources::{Found, Full, Stated, lacking, unknown_maker};
use crate::error::PackError;

/// The open data's commit the tables are read at.
const COMMIT: &str = "8d276f6462a4513eeeeefe00a4880744a9dccd42";

/// A table's URL at `COMMIT`.
fn table_url(table: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/NationalGalleryOfArt/opendata/{COMMIT}/data/{table}.csv"
    )
}

/// A table's name in the cache.
pub fn table_name(table: &str) -> String {
    format!("nga/{table}-{}.csv", &COMMIT[..8])
}

/// An object's page at the gallery (the address Wikidata gives its artwork ids).
fn page(object: &str) -> String {
    format!("https://www.nga.gov/collection/art-object-page.{object}.html")
}

/// What the tables say of the objects: each one's title and attribution, and its primary
/// picture's IIIF service and whether it is open access, by object id.
pub struct Tables {
    objects: HashMap<String, [String; 2]>,
    pictures: HashMap<String, Primary>,
}

/// An object's primary picture: the first in sequence.
struct Primary {
    sequence: u32,
    iiif: String,
    open: bool,
}

impl Tables {
    /// The tables at `COMMIT`, through the cache (about 90 MB each).
    ///
    /// # Errors
    /// `Fetch` when a table cannot be downloaded or lacks a column it is read by.
    pub fn load(fetcher: &Fetcher) -> Result<Self, PackError> {
        let mut tables = Self {
            objects: HashMap::new(),
            pictures: HashMap::new(),
        };
        let url = table_url("objects");
        let bytes = fetcher.get(&table_name("objects"), &url)?;
        let mut rows = Rows::new(&bytes);
        let [id, title, attribution] = rows.columns(["objectid", "title", "attribution"], &url)?;
        for row in rows {
            let pick = |column: usize| row.get(column).cloned().unwrap_or_default();
            (tables.objects).insert(pick(id), [pick(title), pick(attribution)]);
        }
        let url = table_url("published_images");
        let bytes = fetcher.get(&table_name("published_images"), &url)?;
        let mut rows = Rows::new(&bytes);
        let names = [
            "depictstmsobjectid",
            "viewtype",
            "sequence",
            "iiifurl",
            "openaccess",
        ];
        let [object, view, sequence, iiif, open] = rows.columns(names, &url)?;
        for row in rows {
            let pick = |column: usize| row.get(column).map_or("", String::as_str);
            if pick(view) != "primary" {
                continue;
            }
            let primary = Primary {
                sequence: pick(sequence).parse().unwrap_or(u32::MAX),
                iiif: pick(iiif).to_owned(),
                open: pick(open) == "1",
            };
            let key = pick(object).to_owned();
            if (tables.pictures.get(&key)).is_none_or(|kept| primary.sequence < kept.sequence) {
                tables.pictures.insert(key, primary);
            }
        }
        Ok(tables)
    }

    /// Object `object` as the tables describe it: CC0 when its primary picture is open access.
    ///
    /// # Errors
    /// `Fetch` when the tables hold no such object or no primary picture of it.
    pub fn found(&self, object: &str) -> Result<Found, PackError> {
        let url = table_url("objects");
        let [title, attribution] =
            (self.objects.get(object)).ok_or_else(|| lacking(&url, "such object"))?;
        let primary = (self.pictures.get(object))
            .ok_or_else(|| lacking(&table_url("published_images"), "primary picture of it"))?;
        let creator = match attribution.trim() {
            "" => unknown_maker(None),
            name => name.to_owned(),
        };
        Ok(Found {
            title: title.trim().to_owned(),
            credit: creator.clone(),
            creator,
            page: page(object),
            stated: if primary.open {
                Stated::Licence(Licence::Cc0)
            } else {
                Stated::Other("rights restricted, fair use only (openaccess 0)".to_owned())
            },
            picture: format!("{}/full/!2048,2048/0/default.jpg", primary.iiif),
            collection: "National Gallery of Art, Washington",
            changes: "resized",
            headers: &[],
            full: Full::Iiif {
                base: primary.iiif.clone(),
                most: FULL_SIDE,
                only_most: false,
            },
        })
    }
}

/// The records of a CSV table (RFC 4180: comma-separated, a quoted field may hold commas, line
/// breaks and doubled quotes), its first the column names.
struct Rows<'a> {
    rest: &'a [u8],
}

impl<'a> Rows<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { rest: bytes }
    }

    /// The places of the columns `names` in the header row, read off.
    fn columns<const N: usize>(
        &mut self,
        names: [&str; N],
        url: &str,
    ) -> Result<[usize; N], PackError> {
        let header = self.next().unwrap_or_default();
        let mut places = [0; N];
        for (place, name) in places.iter_mut().zip(names) {
            *place = (header.iter().position(|column| column == name))
                .ok_or_else(|| lacking(url, &format!("column {name}")))?;
        }
        Ok(places)
    }
}

impl Iterator for Rows<'_> {
    type Item = Vec<String>;

    fn next(&mut self) -> Option<Vec<String>> {
        if self.rest.is_empty() {
            return None;
        }
        let bytes = self.rest;
        let (mut fields, mut field, mut quoted, mut at) = (Vec::new(), Vec::new(), false, 0);
        while let Some(&byte) = bytes.get(at) {
            at += 1;
            match (quoted, byte) {
                (true, b'"') if bytes.get(at) == Some(&b'"') => {
                    field.push(b'"');
                    at += 1;
                }
                (true, b'"') => quoted = false,
                (false, b'"') => quoted = true,
                (false, b',') => fields.push(text(&mut field)),
                (false, b'\n') => break,
                (false, b'\r') => {}
                (_, byte) => field.push(byte),
            }
        }
        fields.push(text(&mut field));
        self.rest = &bytes[at..];
        Some(fields)
    }
}

/// The bytes of a field as text, taken out of `field`.
fn text(field: &mut Vec<u8>) -> String {
    let text = String::from_utf8_lossy(field).into_owned();
    field.clear();
    text
}

#[cfg(test)]
mod tests {
    use super::Rows;

    #[test]
    fn a_table_reads_quoted_fields_with_commas_breaks_and_quotes() {
        let table = b"id,title,note\r\n1,\"Mill, The\",\"two\nlines \"\"quoted\"\"\"\n2,,last";
        let rows: Vec<Vec<String>> = Rows::new(table).collect();
        assert_eq!(
            rows,
            [
                vec!["id", "title", "note"],
                vec!["1", "Mill, The", "two\nlines \"quoted\""],
                vec!["2", "", "last"],
            ]
        );
    }
}
