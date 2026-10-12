//! Opening an older library (DECISIONS D49, S50): the schema steps run one artwork at a time, on
//! a copy of its records in memory (`steps::run_guarded`), from the schema its head is marked
//! with. An artwork's upgrade is kept only when its records then open as an artwork (the
//! decoding opening it uses, `read::artwork`): a step that panics, or a record a step could not
//! read (cut short, a wrong tag), leaves that artwork's records exactly as they were, its head
//! still marked with its own schema, and the other artworks are upgraded. The toolbox's steps
//! run on their own as well; a toolbox they fail on is kept as it was and read as unreadable
//! (its defaults). Everything that upgraded is written in one batch with the library's new
//! schema, so the library opens upgraded or exactly as it was.
//!
//! An artwork kept so is listed as needing repair (`read::list`), refuses to open with
//! `NotUpgraded`, and saves as an OpenPaint file of its own schema (`artwork_file.rs`); a later
//! build's library upgrade tries it again from that schema.

use std::collections::BTreeMap;

use openpaint_app::{PortError, ProjectId};

use crate::batch::Batch;
use crate::fills::fills_prefix;
use crate::library::{Open, device};
use crate::read;
use crate::records::{
    CLOCK, NEXT, SCHEMA, SCHEMA_KEY, decode_id, encode_schema, head_key, head_schema,
    strokes_prefix,
};
use crate::steps::{Old, Records, Stepped, run_guarded};

/// Records by key, in order.
type Held = BTreeMap<Vec<u8>, Vec<u8>>;

/// Upgrades the library from schema `from`, each artwork on its own; see the module.
pub(crate) fn upgrade(open: &Open, from: u32) -> Result<(), PortError> {
    let mut library = Held::new();
    let mut batch = Batch::new(open);
    let (mut upgraded, mut kept) = (0, Vec::new());
    for id in projects(open)? {
        match project(open, id)? {
            Some(stepped) => {
                upgraded += 1;
                for (key, value) in stepped.projects {
                    batch.insert(&open.projects, key, value);
                }
                merge(&mut library, stepped.library);
            }
            None => kept.push(id.get()),
        }
    }
    merge(&mut library, toolbox(open, from)?.library);
    library.insert(SCHEMA_KEY.to_vec(), encode_schema(SCHEMA));
    for (key, value) in library {
        batch.insert(&open.library, key, value);
    }
    batch.commit()?;
    log::info!(
        "library upgraded from schema {from}: {upgraded} projects upgraded, kept as they were: \
         {kept:?}"
    );
    Ok(())
}

/// Every project with a head, by id.
fn projects(open: &Open) -> Result<Vec<ProjectId>, PortError> {
    let next = match open.library.get(NEXT).map_err(device)? {
        Some(next) => decode_id(&next, "next project id")?,
        None => 1,
    };
    let mut ids = Vec::new();
    for id in (1..next).map(ProjectId::new) {
        if open.projects.get(head_key(id)).map_err(device)?.is_some() {
            ids.push(id);
        }
    }
    Ok(ids)
}

/// What upgrading project `id` from the schema its head is marked with writes, when its records
/// then open as an artwork; `None` (logged) when they are kept as they were.
fn project(open: &Open, id: ProjectId) -> Result<Option<Stepped>, PortError> {
    let records = records_of(open, id)?;
    let head = records.get(&head_key(id)).cloned().unwrap_or_default();
    let from = match head_schema(&head) {
        Some(SCHEMA) => return Ok(Some(Stepped::default())),
        Some(schema) if (1..SCHEMA).contains(&schema) => schema,
        _ => {
            log::error!(
                "project {}: no schema to upgrade from; kept as it was",
                id.get()
            );
            return Ok(None);
        }
    };
    let under = |prefix: Vec<u8>| -> Records {
        let after = records.range(prefix.clone()..);
        let within = after.take_while(|(key, _)| key.starts_with(&prefix));
        within
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    };
    let old = Old {
        heads: vec![(id, head.clone())],
        strokes: under(strokes_prefix(id)),
        fills: under(fills_prefix(id)),
        editor: None,
    };
    let Some(stepped) = run_guarded(from, old) else {
        log::error!(
            "project {}: a schema step from {from} failed; kept as it was",
            id.get()
        );
        return Ok(None);
    };
    let mut upgraded = records;
    upgraded.extend(stepped.projects.clone());
    match read::artwork(&upgraded, id) {
        Ok(_) => Ok(Some(stepped)),
        Err(error) => {
            log::error!(
                "project {}: its records do not open after the steps from schema {from} \
                 ({error}); kept as they were",
                id.get()
            );
            Ok(None)
        }
    }
}

/// Project `id`'s head, stroke and edit records (not its thumbnail or versions).
fn records_of(open: &Open, id: ProjectId) -> Result<Held, PortError> {
    let mut records = Held::new();
    if let Some(head) = open.projects.get(head_key(id)).map_err(device)? {
        records.insert(head_key(id), head.to_vec());
    }
    for prefix in [strokes_prefix(id), fills_prefix(id)] {
        for guard in open.projects.prefix(prefix) {
            let (key, value) = guard.into_inner().map_err(device)?;
            records.insert(key.to_vec(), value.to_vec());
        }
    }
    Ok(records)
}

/// What the toolbox's steps from `from` write; nothing (logged) when they fail.
fn toolbox(open: &Open, from: u32) -> Result<Stepped, PortError> {
    let old = Old {
        heads: Vec::new(),
        strokes: Vec::new(),
        fills: Vec::new(),
        editor: read::editor(open)?,
    };
    Ok(run_guarded(from, old).unwrap_or_else(|| {
        log::error!("the toolbox's schema steps failed; it is kept as it was");
        Stepped::default()
    }))
}

/// Adds one run's `records` to the library's. Step 1 -> 2 starts the change clock at the newest
/// project's id, which each artwork's run sees alone: the clock keeps the largest.
fn merge(library: &mut Held, records: Held) {
    let clock = |value: &[u8]| decode_id(value, "change counter").ok();
    for (key, value) in records {
        let later = || library.get(&key).and_then(|held| clock(held)) >= clock(&value);
        if key != CLOCK || !later() {
            library.insert(key, value);
        }
    }
}
