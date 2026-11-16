# Archived decisions, 2026-11

## Archived 2026-11-16 (S147: D113 folded into D123, which held its results since S145; and historical fragments, as they stood)

- D97's reason went on: "The rule of 2026-10-23 waited whenever a stroke's frame before was unfinished, and a light stroke's frame is, merely submitted late in its vsync: every stroke at a view tier drew every other vsync (the author's pen on S134's build, 2026-11-15: 301 events in 150 frames, frame work p50 6.1 ms, the GPU's work as at fit)."
- D96's display cache read "(S128: that copied and read back 73 MB a stroke on a day's sheet, 61 to 68 ms; after a kill …)" and its idle " (S133: a real pen hovers between strokes, so nothing was idle and its strokes took them cold in their frames)".
- D106's reason began "D103's ground model outlined 60 of 126;".
- D115's reason ended "Chosen by the main session 2026-11-11, the author's to overrule: dots at zero, a first lesson of three, a redo never first." (STATUS keeps it among the choices made for the author).

## D113 2026-11-09 A result from each kind: clean, near, redo, the eye's, or incomplete (S119)
- Decision: every comparison gives `Verdict::App(Outcome)`, `Eye` or `Incomplete` through `Compared::verdict`, required and without a fallback. Clean is exactly what Compare marks nothing for; near is a band named as a constant beside each check (`NEAR_*`, `AREAS_CLEAN`); incomplete is a copy that does not yet cover its exercise (each kind's coverage test) or a reading not yet arrived. Who judges: D123 (amended 2026-11-16, S113 and S145). Shown (S122, D115): after Compare a kind the eye judges asks Accurate, Almost or Again once (`PracticeIntent::Judge`, `Laid::judged`; refused before Compare, where the app judges and a second time); Next never asks and leaves none; the app's result is shown ringed; the first lesson scores nothing; every finished lesson's results stay marked under its pairs on the sheet.
- Reason: the author (2026-11-09): an exercise ends in one result, never a number shown. Consequences: a new kind must give a verdict; the near bands but the colours' rest on one sample or none and are revisited as records gather; a result counts for the dots whoever judged it (`Learner::take`).

## Archived 2026-11-05 (S105: superseded and historical fragments, as they stood; the decisions themselves unchanged)

- D102's reason ended "; STATUS pending 28, option (c)." (that pending decision was settled and has left STATUS).
- D103's consequences began "60 of the official pack's 126 pictures have an outline;" (superseded by D106: 126 of 126 outlined).
- D107's consequences began "s80, s82 and s83 tap Next once more; S88's keep-off of the original's frame retired with that frame (S97);" (the scenarios' change at S84 and S97's retirement, history).

## Archived 2026-11-05 (S101: D105's generated colour combinations superseded by crops of real pictures, and historical fragments, as they stood)

- D105's first bullet, superseded by S101's crops (kind 18), the generator kept for kind 5's records, the How's example and the validation: "Colour combinations (kind 5, `practice/src/areas*.rs`): 4 to 6 areas of a field (bands, patches of a power diagram, blocks) under a slow warp, soft or hard edged; colours related to a key, one contrasting, half or more under HSV saturation 0.3, every neighbour pair at least Oklab 0.025 apart, one pair at most `WEAK` 0.06 and one at least `STRONG` 0.15; laid again from the seed's sequence until kept. A 128² map holds each area's core (6 cells and any soft edge's reach inside its edges)."
- D105's value scales read "(kind 6, `scale.rs`; S97, 2026-10-30, the author: a How of 5 cells beside rows of up to 7)" and their example's order "(S97's review: a five-cell row read against nine cells' "the 5th" confused)".
- D105's metrics read "(35; 32, off in HSV value, retired and passed over)"; its consequences ended "; a 1.5.0 scale's mean off is not shown again."
- D105's pens began "(S100, 2026-11-05; the author, new to painting: Opaque Paint's opacity followed pressure, 30 % at the lightest, so a light stroke showed a colour other than the one picked, and its bristly edge was texture V大 keeps for later)" and their Correct "(D99; the Pencil and Ink Pen as 1.17.0 shipped them)".
- D105's validation listed the reading's cases: "the picture itself, a copy with its edges 12 px off and one a third strays read their exact colours; a copy lighter all over makes nothing strong; a weak pair pushed 0.22 apart is caught there only; greys 128/140 weak, 128/180 strong."
- D106's reason read "IS-Net not taken: its repository licenses only code and metric, its dataset has its own terms." and its consequences "the first build downloads 176 MB and a debug build takes about 20 minutes;".

## Archived 2026-11-06 (S107: room for D105's key shifts; facts ARCHITECTURE and STATUS hold, and historical fragments, as they stood)

- D98: "- Where: generators, metrics, records and the sheet's layout are the pure crate `openpaint-practice` (no dependency), beside core: practice is not artwork semantics."
- D99's lines read: "- Lines (the author after the first sessions: a number that doesn't match what the eye sees misleads): no numbers on the card, in the summary or in the records; `line_score.rs`, key points and the wobble smoothing are deleted. Metric codes 1 to 8 (1.1.0's line metrics) are retired, never reused, and passed over when read, as any unknown code is: old records read unchanged. The strokes stay in the sheet for a later study."
- D99: "- Tools by the exercise: a card beside the frame at the right (a line's pens with undo and redo; a colour's wheel, the colour panel's own, and Fill frame, which fills the frame's region at once); All tools after Gallery brings the full interface back; session state, not stored."
- D102: "- Stored (format 1, additive): the warm-up's count, a u32 after the results' tail; absent, none."
- D105: "- Validated on known cases (`practice/tests/areas.rs`: a copy's reading; `crops.rs`: the found answer on 72 pictures)."
- D108's hue read: "- Hue: no gate (1.6.0 called a colour under 0.06 of RGB chroma grey and said nothing of its hue): the hue part fades with chroma as the eye's does, so a dark olive (42, 46, 37) against a dark brown (50, 47, 42) reads ΔL +1.3, ΔC −0.7, ΔH −1.0; the wheel rings a colour's hue wherever its Oklab chroma reaches `HUELESS` 0.005, below which an 8-bit grey's hue is rounding."
- D108's stored read: "- Stored (format 1, additive): codes 36 to 41, a colour's first and final ΔL, ΔC, ΔH; 21 to 23 and 25 to 27 (HSV hue behind the gate, saturation, value) retired and passed over. The radar (`judgement.rs`): hue, chroma and lightness in place of hue, saturation and value, each 1 - part/`PART_SPAN` 0.05 clamped, then dark, greyish (ΔE against 0.10) and first pick to final as before, over the earlier sessions' dashed; the summary's rows unchanged."
- D109: "- Motion: a finish's dot inking in (two rings spreading) and a seal stamping down are D44's acknowledge on the OPEN spring, each waiting for the card to grow."
- D110: "- Tiles: fixed line drawings in the UI (`components/kind_tile.rs`), ink once the kind was compared anywhere, pencil before, ringed while a drill is unfinished; Shape without a pack's picture says where to import one. The options' first page in two columns where the screen holds 800 points."

## Archived 2026-11-07 (S109: a validation report, kept in STATUS; the decision unchanged)

- D105's key shift validation read: Validated on the official 61 crops (`openpaint-harness key-shifts`, sheets judged by eye) against shifts by rule (`key_shift_rule.rs`, also the How's) and by a simulated hand, 61 of 61 unmarked each; lightness 0.14 and 0.09 off, an area unmoved or a quarter moved, two within 0.025 marked; 0.06, three fifths, 0.05 not.

## Archived 2026-11-07 (S109b: finished, its rules now in `kinds/combined_lines/`, `sessions/session.rs` and AGENTS.md's practice-test chore; room for D111)

## D102 2026-10-23 Combined lines, a warm-up from the kinds met, a test session beside the author's (debug builds)
- Combined lines (kind code 3, `practice/src/combined*.rs`): two to four lines joined (open or closed), branched, crossing or near; key points laid first, each line a single line's curvature laid between two; held to rules in frame units (a line at least 0.16, a gap at least 0.05, near lines 0.04 to 0.2 apart, corners and crossings 30 degrees or more from straight, one crossing away from the ends), made again from the sequence until kept. Compared as lines (D99); the aligned fit lays the group as one shape, and of the whole, the whole without one line and each line alone fits the part leaving the least between it and the copy both ways, so a group compared before every line is drawn lands on the lines drawn.
- Session (stage 1, `practice/src/session.rs`): a warm-up of `WARM_UP` 4 from the kinds compared in any earlier session, the families met taken in turn (lines, combined lines, colours), each kind by the seed, sorted by family; then 6 lines, 4 combined (each arrangement), 6 colours: about 28 minutes from the second session (S75's estimates, a group about 2), 23 for the first, whose first colour stays the eleventh pair. Instructions show by themselves for a family neither compared before nor warmed up with.
- Test session (`cargo device-input practice-test start|end`, `runtime/src/practice_test.rs`): the session Start practice would start now (or, `start <stage> <n>`, as that stage's n-th would be, so stage 2's free boxes are reached; S88), started from the gallery beside any open one and named by id, sheet, seed and start, kept on the dev machine; `end` removes the session of that name, record and sheet in one batch (`ProjectStore::remove_practice`), nothing else; the next session takes its id again.
- Reason: the author's G9 (V大's W1 combined lines; the warm-up from what was learnt). Consequences: a removal moves `clock` and `next` on and nothing else; the fit tries up to nine parts (on the fit worker, D103).

## Archived 2026-11-07 (S110: room for D112; their rules now stated in ARCHITECTURE sections 1, 7 and 8 and AGENTS.md and held by repo-check, and D19's "screenshots never compared" superseded since S109 by pixel comparisons when behaviour must stay)

## D10 2026-10-02 Errors are types; product crates have no cargo features
- Decision: `CoreError`, `AppError`, `PortError`, `RenderError`, `RuntimeError`; `Result<_, String>` only in JNI, system APIs and xtask. No `cfg(feature)` in product crates; diagnostics and automation live in separate crates assembled only by composition roots. Reason: the previous project's `String` errors and 379 `cfg` sites made four build combinations and hid diagnostics in product code. Consequences: `cargo repo-check` rejects `[features]` and `cfg(feature`; the platform layer turns typed errors into user-facing messages.

## D19 2026-10-02 Scenario checkpoints compare canvas hashes; references stay untracked
- Decision: a checkpoint passes when the canvas readback (artwork pixels, no UI) hashes to the value in the scenario file; screenshots are for looking at, never compared; the diff reference is the last pass on this machine (`artifacts/scenarios/<name>/`, untracked). Reason: canvas pixels come from the renderer's own formulas, egui's text edges differ by backend. Consequences: a slice that changes pixels on purpose updates the hash in the same commit after looking at the PNG; UI changes are checked by screenshots and GPU tests on specific pixels.

## D48 2026-10-04 Scenarios name controls by their accessible label; device chores are xtask commands
- Decision: every shared component registers its label and rectangle in a per-pass `ControlMap` (always on); scenario steps, runtime tests and `device-input tap` name the control (`name #n` when shared), pixels only on the canvas, and pixels on a control fail the run with the name to write. Device work goes through `cargo device-*` (adb with explicit arguments, never a host shell; restore verified byte for byte).
- Reason: layout changes rewrote hard-coded taps in a dozen files; a hand-rolled device chore once wiped the author's library.
- Consequences: a new component registers its control; a label change is a scenario change; slider and wheel positions are offsets from the control's middle.

## Archived 2026-11-12 (S122: the stages, their sessions and drills superseded by the path, D115, and the home, D116; as they stood)

## D110 2026-10-28 Drills: one kind, a lesson `Chosen` on the day's sheet, never a session
- Decision (the author: try every kind one by one, and practise one on purpose, 专项训练): the practice options' Practice section lists each ready stage (its row starts a session) with its kinds as tiles (`practice::kinds_of`); a tile starts a drill of the kind (`practice::drill`) or continues the one left unfinished. A drill holds as many exercises as its kind's description says (`drill`; shapes' tiers in turn, the user's own last; none without a composition, value masses only of pictures with them), variants in turn by its number among its kind's; no warm-up; its How first every time it opens; One more starts the kind's next.
- Stored as a lesson `Chosen` with the kind on the day's sheet (D98, D114), its number derived among its kind's drill lessons. Counted: its time and day in the book, the kinds met (warm-ups draw on them, tiles ink), the pictures shown, the seals of lines, colours and time; never as a session (Today, the road, sessions done, a stage's seals, the first picks, the lines shown). Its summary sets it beside the earlier finished drills of its kind only.
- Reason: V大 drills one skill at a time, and a drill chosen by the user must not move the course on. Consequences: the harness gives every sheet one seed, so its drills of a kind repeat their exercises.

Fragments of other decisions, each replaced by a phrase on the path:

- D98's picked read "(`Chosen`: a drill's kind, a copy's picture; `Full { stage, warm_up }`: a stage's session)".
- D103's title held "stage 2," and its stage bullet read: "- Stage 2: 4 shapes (tiers harder as sessions go, `session.rs`; the last the user's own as D105 says; pictures shown before wait while fresh ones remain), its warm-up and the rest of its mix in D107. The stage is the last session's; the Today card offers the next once `STAGE_SESSIONS` are done, the options any other ready stage at once."
- D105 read: "`FIRST_CELLS` in the first `FIRST_SCALES` a beginner meets in sessions and drills (`Before::scales`)"; "It counts in no stage, warm-up, `used` or summary"; "A stage-2 session's last shape is one of them of the session's hardest tier or easier (fresh ones first)"; "the dialog counts the sessions and copies that used it"; "leans in turn (warm pure, cool grey, warm grey, cool pure; ...)"; "Stage 5: a warm-up of three, three key shifts; a drill of two; without crops the row and tile refuse."
- D107 read "(`Family::constructed`: ...)" and: "- Stage 2 (`session.rs`): a warm-up of `WARM_UP_TWO`, at most one of the `LONG` kinds and the `QUICK_TWO` in turn by number; then 4 shapes (D103), a combination, a scale, and a box and an ellipse in odd sessions, free boxes in even ones, variants in turn; about 29 to 31 minutes." and "- Stage 3 (`session_three.rs`): a warm-up of three as stage 2's (solids never in it), four estimates (each, then one in turn), `TRICKS` tricks and `SOLIDS` solids in turn, shapes of `TIERS` (none without a pack), a combination; about 30 minutes. Stage 4 (`session_four.rs`): a warm-up of three, `OBJECTS` objects, then a division and a whole copy, in odd sessions value masses in its place where a picture has them, of `COMPOSITION_TIERS` in turn (D108; a picture of the last `RECENT` sessions only when none other is left), else a solid, a shape of `TIERS` and a combination in turn."
- D108 read "the summary adds no note"; "its week in V大's course"; "Again replaying"; "The radar: hue, chroma, lightness (each 1 − part/`PART_SPAN`), dark, greyish, first pick to final."; its consequences began "sessions before 1.6.1 count in the radar's last three axes only;".
- D109's seals read "counts of sessions, a stage's sessions, lines, colours and hours" and "Each lesson's Finish stamps".
- D111 read "its families' `Facts`, its original, How, drill, givens", "any `Family` or `Original` variant named" and "a stage plan and a scenario"; D112's hooks "`CanvasHook`, `GalleryHook`"; D114's start "a session or drill started from the gallery or the options", "never continued from the gallery", its consequences ended "; two short sessions on one day can share a sheet."

## Archived 2026-11-14 (S124: room for D117 and D118; D01's stack is stated by ARCHITECTURE's opening and crate map, the full lesson went with S124, and historical fragments, as they stood)

## D01 2026-10-02 Stack: Rust 2024, wgpu, egui, one workspace
- Decision: core, rendering, application, storage and UI are all Rust; GPU through wgpu; UI through egui; Android is the only product host; the development machine runs a headless harness, never a desktop window (ARCHITECTURE). Reason: the previous project proved this stack on the target tablet (pressure, palm rejection, layer compositing, smudge). Consequences: renderer state never lives in egui's type map; egui appears only under `render/src/adapter/`.

Fragments of other decisions:

- D79's version bullet read "a minor per stage (2026-11-09: a version a slice re-took 18 screenshots and rebuilt every crate)".
- D97's reason read "every third at rest (panels opened at 36 to 48 % of the vsyncs, on the glass 25 ms apart);".
- D98's model read "each picked (`Chosen`: a skill's lesson or a review, a copy's picture; `Full { stage, warm_up }`: a unit's full lesson, a stage's session before S122 read as that unit's; `First`: the first lesson; D115)" and its stored bullet "1.21.6's session records were converted once (S120c); S120e deleted the step."
- D103's frames read "Frames (the author on the tablet, 2026-10-30: a picture not square lay in the square frame with its own margins, so its edge and the copy's square disagreed):".
- D115's skills ended "; a full lesson is a warm-up of three (earlier units' skills first), then each skill of its unit."; its stored bullet read "an old `Full { stage }` reads as that unit's full lesson. Stages, their plans, the 15-session gate, drills, the warm-up rotation and the summary are gone (D110 archived)."

## Archived 2026-11-14 (S123: fragments as they stood; the decisions themselves unchanged)

- D99's lines bullet ended "; codes 1 to 8 retired; the strokes stay for a later study."; its compare bullet read "(S88, the author: 12 px hid it; a shape's outline, a box's or an ellipse's answer and every red mark over a copy alike)" and "on the copy's raw pen samples (resampled, a known move and size no longer comes back within 1e-6)".
- D107's found bullet read "(`DOUBLE_*`: the author's redraws, a few px and degrees off, read as two at 4 px)"; its How bullet "has no round part (a post's step is 29 lines at least, unreadable on the card)".
- D108's cards bullet began "Cards (the author on the tablet: they read like code comments, numbers said as sentences):" and said "in each family's How".
- D111 before S123: its description was one trait `Described` with the original an enum `Original` (12 variants), a family list `Family` (27) keying facts, How, tiles and words, and `Tools` and `Picks` as enums; repo-check refused outside `crates/practice/` an arm over `Kind`, any `Original` variant and the name `Family`.

## Archived 2026-11-14 (S126: room for D119; D112's mode slot superseded by it, and historical fragments, as they stood)

- D112's heading ended "record spaces, a mode slot and hooks"; its flow bullet read "a mode slot on the session (`Mode`: raw, lands, clip, style, amended, laid; `lend` with `Lent`), reached by its type in one runtime accessor; practice's intents outside `Intent`, applied after the frame's painting ones; a mode's pickers in the gallery's turn (`Gallery::choose`, `FileJob::Mode`);" and its hooks "each called where its part is drawn (the glyph atlas follows draw order)".
- D98's heading carried "(S120c 2026-11-09)"; its model bullet "`Full`, S122's full lesson, only read;" and ended "the open one's flow is `Session`'s `PracticeSheet`."; D96's display cache named `runtime/src/canvas_cache.rs`.
- D85's test art: "import (the instrumentation's shell reads the file; the picker is skipped)". D97: "`Wanted`; replaces D96's frame rule", "for up to `PATIENCE` calls (D96's)", "a call earlier (D96's)", "the islands and panels there are (an open panel cost 4 to 11 ms of GPU a frame)".
- D99: "an error of a pen's width shows (S88; every red mark over a copy alike)". D103: "(its alpha scaled, `practice::faded_ground`; a trial put to the author)".
- D105: "Colour combinations (2026-11-05: generated ones varied too little; V大 copies a corner of an abstract painting or a rusted old thing)", "(35; 32 retired)", "Pens (2026-11-05; the author: a light stroke of Opaque Paint showed another colour than the one picked)", "Key shifts (2026-11-06, V大's 变调 ...)".
- D107: "One frame (2026-10-30, the author: the original's frame only repeated the question; `Facts::constructed`: a value scale, a box, free boxes, an ellipse; then, \"the same way\", the five tricks and three solids)", "Shown (the author: every stroke drawn on dashed was clutter; every geometric kind alike", "an object's kept off its picture (run-ons crossed onto the chrome on the tablet)", "(kind 13, `solid*.rs`, 2026-10-30)", "(kind 14, `object*.rs`, 2026-10-30)".
- D108: "ΔH (21 to 23 and 25 to 27 retired)", "How (2026-10-28; the author, new to drawing: \"tell me how to practise\")", "no week (S122: the path has units)", "Level colours (2026-10-30)", "half at first (35 % read as a pale wash on a dark painting on the tablet)", "Value masses (2026-10-30; V大's 黑白灰 ...)".

## Archived 2026-11-14 (S127: room for D97's measure, as they stood)

- The header: "\"The previous project\" is draweave (2026-07 to 2026-10), the predecessor this repository replaces." (no decision used the phrase).
- D97's consequences: "pans and pinches too (the zoom bench's pinches skip nothing)", measured before S127's count.
