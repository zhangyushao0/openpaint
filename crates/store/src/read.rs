//! Reading the library: a project's artwork from its head, stroke and fill records (in the
//! library or in an OpenPaint file's records), the catalogue for the gallery, and the small
//! library values.

use std::collections::BTreeMap;

use fjall::Keyspace;
use openpaint_app::{PortError, Project, ProjectId, ProjectSummary, Title};
use openpaint_core::{Artwork, ArtworkParts, EditParts, MAX_NESTING, NestedParts, StrokeParts};

use crate::catalogue::{Entry, decode_entry, decode_thumbnail};
use crate::editor::EDITOR;
use crate::fills::{decode_record, fill_layer_prefix};
use crate::library::{Open, device};
use crate::nested::Counts;
use crate::records::{
    CLOCK, ENTRY, LAST, decode_head, decode_id, decode_stroke, entry_key, entry_project, head_key,
    layer_prefix, mask_slot, stroke_index, thumbnail_key,
};
use crate::versions;

/// What `Source::each` hands every record to: its key and value.
pub(crate) type Visit<'a> = dyn FnMut(&[u8], &[u8]) -> Result<(), PortError> + 'a;

/// Where a project's records are read from: the library's `projects` keyspace, or an OpenPaint
/// file's records held in memory under the same keys.
pub(crate) trait Source {
    /// The value stored under `key`.
    fn value(&self, key: &[u8]) -> Result<Option<Vec<u8>>, PortError>;

    /// Hands `each` every record under `prefix`, in key order.
    fn each(&self, prefix: &[u8], each: &mut Visit<'_>) -> Result<(), PortError>;
}

impl Source for Keyspace {
    fn value(&self, key: &[u8]) -> Result<Option<Vec<u8>>, PortError> {
        Ok(self.get(key).map_err(device)?.map(|value| value.to_vec()))
    }

    fn each(&self, prefix: &[u8], each: &mut Visit<'_>) -> Result<(), PortError> {
        for guard in self.prefix(prefix) {
            let (key, value) = guard.into_inner().map_err(device)?;
            each(&key, &value)?;
        }
        Ok(())
    }
}

impl Source for BTreeMap<Vec<u8>, Vec<u8>> {
    fn value(&self, key: &[u8]) -> Result<Option<Vec<u8>>, PortError> {
        Ok(self.get(key).cloned())
    }

    fn each(&self, prefix: &[u8], each: &mut Visit<'_>) -> Result<(), PortError> {
        let under = self.range(prefix.to_vec()..);
        for (key, value) in under.take_while(|(key, _)| key.starts_with(prefix)) {
            each(key, value)?;
        }
        Ok(())
    }
}

/// The project open when the app was left, if any.
pub(crate) fn last(open: &Open) -> Result<Option<Project>, PortError> {
    let Some(last) = open.library.get(LAST).map_err(device)? else {
        return Ok(None);
    };
    let id = ProjectId::new(decode_id(&last, "last project")?);
    let project = project(open, id).map_err(|error| match error {
        // `last` names a project the library does not hold.
        PortError::Missing(_) => PortError::Corrupt {
            what: "artwork head",
        },
        other => other,
    })?;
    Ok(Some(project))
}

/// Project `id` with every stroke.
pub(crate) fn project(open: &Open, id: ProjectId) -> Result<Project, PortError> {
    let artwork = artwork(&open.projects, id)?;
    // The title is the catalogue's; a missing entry must not keep the artwork from opening.
    let title = entry(open, id).map_or_else(
        |error| {
            log::warn!("project {}: no title: {error}", id.get());
            Title::untitled()
        },
        |entry| entry.title,
    );
    log::info!("project {} opened", id.get());
    Ok(Project { id, title, artwork })
}

/// The artwork of project `id` as `source` holds it, checked whole.
pub(crate) fn artwork(source: &dyn Source, id: ProjectId) -> Result<Artwork, PortError> {
    let head = source.value(&head_key(id))?.ok_or(PortError::Missing(id))?;
    let head = decode_head(&head)?;
    let mut layers = Vec::with_capacity(head.layers.len());
    for (mut layer, strokes, fills, [mask_strokes, mask_fills]) in head.layers {
        (layer.strokes, layer.edits) = content(source, (id, layer.id), [strokes, fills], 0)?;
        if let Some(mask) = layer.mask.as_mut() {
            let slot = (id, mask_slot(layer.id));
            (mask.strokes, mask.edits) = content(source, slot, [mask_strokes, mask_fills], 0)?;
        }
        layers.push(layer);
    }
    Ok(Artwork::from_storage(ArtworkParts {
        width: head.width,
        height: head.height,
        layers,
        groups: head.groups,
        active: head.active,
        on_mask: head.on_mask,
        next_layer_id: head.next_layer_id,
        next_stroke_id: head.next_stroke_id,
    })?)
}

/// The `strokes` stroke records and `edits` edit records stored under `slot` (a layer's id, a
/// merged layer's or a mask's, `mask_slot`) of project `id`, `depth` nested records deep; each
/// nested record with its own content.
fn content(
    source: &dyn Source,
    (id, slot): (ProjectId, u64),
    [strokes, edits]: [usize; 2],
    depth: usize,
) -> Result<(Vec<StrokeParts>, Vec<EditParts>), PortError> {
    let prefix = layer_prefix(id, slot);
    let strokes = read_records(source, &prefix, strokes, "stroke records", decode_stroke)?;
    let prefix = fill_layer_prefix(id, slot);
    let records = read_records(source, &prefix, edits, "edit records", decode_record)?;
    let mut edits = Vec::with_capacity(records.len());
    for record in records {
        edits.push(match record {
            (EditParts::Nested(nested), Some(counts)) => {
                EditParts::Nested(nested_content(source, id, (nested, counts), depth + 1)?)
            }
            (edit, _) => edit,
        });
    }
    Ok((strokes, edits))
}

/// A nested record's content and its mask's, from the records under its id; a record nested
/// deeper than core allows is damage (a corrupt record could name itself).
fn nested_content(
    source: &dyn Source,
    id: ProjectId,
    (mut nested, [strokes, edits, mask_strokes, mask_edits]): (NestedParts, Counts),
    depth: usize,
) -> Result<NestedParts, PortError> {
    if depth > MAX_NESTING {
        return Err(PortError::Corrupt {
            what: "nested records",
        });
    }
    (nested.strokes, nested.edits) = content(source, (id, nested.id), [strokes, edits], depth)?;
    if let Some(mask) = nested.mask.as_mut() {
        let slot = (id, mask_slot(nested.id));
        (mask.strokes, mask.edits) = content(source, slot, [mask_strokes, mask_edits], depth)?;
    }
    Ok(nested)
}

/// The `count` records under one layer's `prefix` (its strokes or its fills), decoded, which
/// must sit at indices `0..count`.
fn read_records<T>(
    source: &dyn Source,
    prefix: &[u8],
    count: usize,
    what: &'static str,
    decode: fn(&[u8]) -> Result<T, PortError>,
) -> Result<Vec<T>, PortError> {
    let corrupt = PortError::Corrupt { what };
    let mut values = Vec::with_capacity(count);
    source.each(prefix, &mut |key, value| {
        let expected = values.len();
        let in_place = key.len() == prefix.len() + 8 && stroke_index(key) == Some(expected as u64);
        if expected >= count || !in_place {
            return Err(corrupt.clone());
        }
        values.push(decode(value)?);
        Ok(())
    })?;
    if values.len() == count {
        Ok(values)
    } else {
        Err(corrupt)
    }
}

/// Every project in the catalogue, touched last first, with its thumbnail. A thumbnail that
/// cannot be read is left out (it is derived and rewritten when the artwork is next left).
pub(crate) fn list(open: &Open) -> Result<Vec<ProjectSummary>, PortError> {
    let mut entries = Vec::new();
    for guard in open.library.prefix(ENTRY) {
        let (key, value) = guard.into_inner().map_err(device)?;
        let id = entry_project(&key).ok_or(PortError::Corrupt {
            what: "catalogue key",
        })?;
        entries.push((id, decode_entry(&value)?));
    }
    entries.sort_by(|(a_id, a), (b_id, b)| (b.touched, b_id).cmp(&(a.touched, a_id)));
    let mut summaries = Vec::with_capacity(entries.len());
    for (id, entry) in entries {
        let thumbnail = match open.projects.get(thumbnail_key(id)).map_err(device)? {
            Some(bytes) => decode_thumbnail(&bytes)
                .inspect_err(|error| log::warn!("project {}: {error}", id.get()))
                .ok(),
            None => None,
        };
        summaries.push(ProjectSummary {
            id,
            title: entry.title,
            thumbnail,
            versions: versions::listed(open, id)?,
        });
    }
    Ok(summaries)
}

/// Project `id`'s catalogue entry.
pub(crate) fn entry(open: &Open, id: ProjectId) -> Result<Entry, PortError> {
    let bytes = open
        .library
        .get(entry_key(id))
        .map_err(device)?
        .ok_or(PortError::Missing(id))?;
    decode_entry(&bytes)
}

/// The toolbox record's bytes, if one was ever written.
pub(crate) fn editor(open: &Open) -> Result<Option<Vec<u8>>, PortError> {
    let bytes = open.library.get(EDITOR).map_err(device)?;
    Ok(bytes.map(|bytes| bytes.to_vec()))
}

/// The change counter's last value.
pub(crate) fn clock(open: &Open) -> Result<u64, PortError> {
    match open.library.get(CLOCK).map_err(device)? {
        Some(bytes) => decode_id(&bytes, "change counter"),
        None => Ok(0),
    }
}
