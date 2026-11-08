# Archived decisions, 2026-11

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
