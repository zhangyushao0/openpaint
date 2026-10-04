# Archived decisions, 2026-10

Superseded decisions moved here whole from `docs/DECISIONS.md`. Written, never read.

## D24 2026-10-02 S9 keeps schema 1: layers needed no format change

- Decision: S9 ships without a schema 2. Schema 1 (D21) already stores everything S9 edits: the layer order, each layer's name, visibility, opacity and lock, and the active layer in the head; strokes keyed by layer id. The first migration step 1 -> 2 and its frozen sample land with the first slice that changes the format (S11 stores recipe snapshots). Pending the author's confirmation (STATUS).
- Reason: a schema 2 identical to schema 1 would add a migration that changes nothing; adding fields for later slices (alpha lock, blend, groups) would reserve format for features not yet accepted. Proven instead: the schema-1 sample still opens with its three layers, layer edits round-trip through the library, and the tablet's library written by the S8 build opened unchanged in the S9 build.
- Consequences: supersedes D21's line "schema 2 (S9) adds a migration over these records"; ROADMAP's S9 criterion "the schema 1 sample upgrades" is met by the unchanged sample test. D25 settles the pending question: schema 2 came with S10, the first real format change. Superseded by D25.
