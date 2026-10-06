//! Running the schema steps (DECISIONS D49): every step from a stored schema on, each seeing
//! the heads and strokes the step before it wrote. Pure data: the library writes the result
//! when it opens an older library, an OpenPaint file's records are upgraded in memory when it
//! is opened (D62).

use std::collections::{BTreeMap, HashMap};

use openpaint_app::ProjectId;

use crate::editor::EDITOR;
use crate::migrate::{upgrade_1_to_2, upgrade_2_to_3, upgrade_3_to_4, upgrade_4_to_5};
use crate::migrate_6::upgrade_5_to_6;
use crate::migrate_7::upgrade_6_to_7;
use crate::migrate_8::upgrade_7_to_8;
use crate::migrate_9::upgrade_8_to_9;
use crate::migrate_10::upgrade_9_to_10;
use crate::migrate_11::upgrade_10_to_11;
use crate::migrate_12::upgrade_11_to_12;
use crate::migrate_13::upgrade_12_to_13;
use crate::migrate_14::upgrade_13_to_14;
use crate::migrate_15::upgrade_14_to_15;
use crate::migrate_16::upgrade_15_to_16;
use crate::migrate_17::upgrade_16_to_17;
use crate::records::{SCHEMA, head_key};

/// Records by key.
pub(crate) type Records = Vec<(Vec<u8>, Vec<u8>)>;

/// The records as an older schema stored them: every project's head, every stroke record and
/// every fill record, and the toolbox's.
pub(crate) struct Old {
    pub(crate) heads: Vec<(ProjectId, Vec<u8>)>,
    pub(crate) strokes: Records,
    pub(crate) fills: Records,
    pub(crate) editor: Option<Vec<u8>>,
}

/// What the steps write, per keyspace, each key once with its last value.
#[derive(Debug, Default)]
pub(crate) struct Stepped {
    pub(crate) library: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(crate) projects: BTreeMap<Vec<u8>, Vec<u8>>,
}

/// Runs every step from schema `from` (1 to `SCHEMA - 1`) to `SCHEMA` over `old`.
pub(crate) fn run_steps(from: u32, old: Old) -> Stepped {
    let Old {
        mut heads,
        mut strokes,
        fills,
        mut editor,
    } = old;
    let index: HashMap<Vec<u8>, usize> = strokes
        .iter()
        .enumerate()
        .map(|(at, (key, _))| (key.clone(), at))
        .collect();
    let mut stepped = Stepped::default();
    for step in from..SCHEMA {
        let upgrade = match step {
            1 => upgrade_1_to_2(&heads),
            2 => upgrade_2_to_3(&heads, &strokes),
            3 => upgrade_3_to_4(&heads, &strokes, editor.as_deref()),
            4 => upgrade_4_to_5(&heads),
            5 => upgrade_5_to_6(&heads, &strokes, editor.as_deref()),
            6 => upgrade_6_to_7(&heads, &strokes),
            // Only this step reads the fill records, so they are never rewritten twice.
            7 => upgrade_7_to_8(&heads, &fills),
            8 => upgrade_8_to_9(&heads, editor.as_deref()),
            9 => upgrade_9_to_10(&heads),
            10 => upgrade_10_to_11(&heads),
            11 => upgrade_11_to_12(&heads),
            12 => upgrade_12_to_13(&heads),
            13 => upgrade_13_to_14(&heads),
            14 => upgrade_14_to_15(&heads),
            15 => upgrade_15_to_16(&heads),
            _ => upgrade_16_to_17(&heads, editor.as_deref()),
        };
        if let Some((_, value)) = upgrade.library.iter().find(|(key, _)| key == EDITOR) {
            editor = Some(value.clone());
        }
        for (key, value) in &upgrade.projects {
            if let Some(at) = index.get(key) {
                strokes[*at].1.clone_from(value);
            } else if let Some((_, head)) = heads.iter_mut().find(|(id, _)| head_key(*id) == *key) {
                head.clone_from(value);
            }
        }
        stepped.library.extend(upgrade.library);
        stepped.projects.extend(upgrade.projects);
    }
    stepped
}
