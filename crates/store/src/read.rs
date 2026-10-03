//! Reading the library: a project's artwork from its head, stroke and fill records, the catalogue for
//! the gallery, and the small library values.

use openpaint_app::{PortError, Project, ProjectId, ProjectSummary, Title};
use openpaint_core::{Artwork, ArtworkParts};

use crate::catalogue::{Entry, decode_entry, decode_thumbnail};
use crate::editor::EDITOR;
use crate::fills::{decode_edit, fill_layer_prefix};
use crate::library::{Open, device};
use crate::records::{
    CLOCK, ENTRY, LAST, decode_head, decode_id, decode_stroke, entry_key, entry_project, head_key,
    layer_prefix, stroke_index, thumbnail_key,
};

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
    let head = open
        .projects
        .get(head_key(id))
        .map_err(device)?
        .ok_or(PortError::Missing(id))?;
    let head = decode_head(&head)?;
    let mut layers = Vec::with_capacity(head.layers.len());
    for (mut layer, strokes, fills) in head.layers {
        layer.strokes = read_records(open, layer_prefix(id, layer.id), strokes, "stroke records")?
            .iter()
            .map(|bytes| decode_stroke(bytes))
            .collect::<Result<_, _>>()?;
        layer.edits = read_records(open, fill_layer_prefix(id, layer.id), fills, "edit records")?
            .iter()
            .map(|bytes| decode_edit(bytes))
            .collect::<Result<_, _>>()?;
        layers.push(layer);
    }
    let artwork = Artwork::from_storage(ArtworkParts {
        width: head.width,
        height: head.height,
        layers,
        active: head.active,
        next_layer_id: head.next_layer_id,
        next_stroke_id: head.next_stroke_id,
    })?;
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

/// The values of the `count` records under one layer's `prefix` (its strokes or its fills),
/// which must sit at indices `0..count`.
fn read_records(
    open: &Open,
    prefix: Vec<u8>,
    count: usize,
    what: &'static str,
) -> Result<Vec<fjall::Slice>, PortError> {
    let corrupt = PortError::Corrupt { what };
    let mut values = Vec::new();
    for (expected, guard) in open.projects.prefix(&prefix).enumerate() {
        let (key, value) = guard.into_inner().map_err(device)?;
        let in_place = key.len() == prefix.len() + 8 && stroke_index(&key) == Some(expected as u64);
        if expected >= count || !in_place {
            return Err(corrupt);
        }
        values.push(value);
    }
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
