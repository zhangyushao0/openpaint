//! `test-art kinds [n]`: a screen-size test artwork holding every kind of record the raster core
//! draws besides strokes, fills and smudges, each over the place the pile's `n` strokes make
//! dense: Curves (its composite and red channels) and Hue, Saturation, Brightness inside a lasso
//! on the dense layer; a Gaussian Blur and a Sharpen inside a rectangle on a layer of its own; a
//! whole layer turned and scaled (a transform record) and a selection of it moved; a merged layer
//! (Merge Down: a nested record) with strokes after the merge; a group at 85 % holding a Multiply
//! layer and the top layer, which holds a smudge and is active, so the device's `bench strokes`
//! draws inside a group.

use std::f32::consts::TAU;

use openpaint_app::BrushLibrary;
use openpaint_core::{
    Adjustment, Artwork, ArtworkCommand, BlendMode, Blur, Curves, GroupCommand, Hsb, LayerCommand,
    Placement, Sharpen, StrokeTool, ToneCurve,
};

use super::page::{colour, layer, mask, style};
use super::{BRUSHES, Hand, SCREEN, pile, stroke, unit};
use crate::error::HarnessError;

/// The pile's middle, where every layer's strokes cross.
const MIDDLE: [f32; 2] = [SCREEN[0] as f32 / 2.0, SCREEN[1] as f32 / 2.0];

/// The dense layer of `n` pile strokes, then a layer for each kind of record about its middle.
pub(super) fn kinds(library: &BrushLibrary, n: u32) -> Result<Artwork, HarnessError> {
    let mut artwork = pile(library, n)?;
    artwork = adjusted(&artwork)?;
    artwork = layer(&artwork)?;
    artwork = strokes_about(&artwork, library, (40, 11), 520.0)?;
    artwork = filtered(&artwork)?;
    artwork = layer(&artwork)?;
    artwork = strokes_about(&artwork, library, (30, 12), 480.0)?;
    artwork = moved(&artwork)?;
    artwork = strokes_about(&artwork, library, (10, 13), 300.0)?;
    artwork = merged(&artwork, library)?;
    grouped(&artwork, library)
}

/// Curves on the dense layer, then Hue, Saturation, Brightness inside a lasso left of its middle.
fn adjusted(artwork: &Artwork) -> Result<Artwork, HarnessError> {
    let curve = |points: &[[f32; 2]]| {
        ToneCurve::new(points).ok_or(HarnessError::Missing("a test-art curve"))
    };
    let curves = Curves {
        composite: curve(&[[0.0, 0.0], [0.5, 0.62], [1.0, 1.0]])?,
        red: curve(&[[0.0, 0.05], [1.0, 0.92]])?,
        green: ToneCurve::identity(),
        blue: ToneCurve::identity(),
    };
    let artwork = (artwork.adjust_region(None, Adjustment::Curves(curves)))?.artwork;
    let [x, y] = [MIDDLE[0] - 260.0, MIDDLE[1]];
    let lasso = mask(artwork.canvas(), |px, py| (px - x).hypot(py - y) <= 300.0)?;
    let hsb = Adjustment::Hsb(Hsb::new(0.62, 0.65, 0.45));
    Ok(artwork.adjust_region(Some(lasso), hsb)?.artwork)
}

/// A Gaussian Blur of 4.8 px over the whole layer, then a Sharpen at 60 % inside a rectangle.
fn filtered(artwork: &Artwork) -> Result<Artwork, HarnessError> {
    let blur = Adjustment::Blur(Blur::new(4.8));
    let artwork = artwork.adjust_region(None, blur)?.artwork;
    let [x, y] = MIDDLE;
    let inside = |px: f32, py: f32| (px - x).abs() < 380.0 && (py - y + 80.0).abs() < 220.0;
    let rect = mask(artwork.canvas(), inside)?;
    let sharpen = Adjustment::Sharpen(Sharpen::new(0.6));
    Ok(artwork.adjust_region(Some(rect), sharpen)?.artwork)
}

/// The layer turned by 7 degrees and scaled by 1.08 about the middle (a record of the whole
/// layer), then a rectangle of it right of the middle moved 90 px right and 60 up.
fn moved(artwork: &Artwork) -> Result<Artwork, HarnessError> {
    let turn = Placement::around(MIDDLE, 1.08, 0.12, [false, false])
        .ok_or(HarnessError::Missing("a test-art turn"))?;
    let artwork = artwork.place_region(None, turn)?.artwork;
    let [x, y] = [MIDDLE[0] + 240.0, MIDDLE[1] - 40.0];
    let lift = mask(artwork.canvas(), |px, py| {
        (px - x).abs() < 200.0 && (py - y).abs() < 150.0
    })?;
    let shift = Placement::moved([90.0, -60.0]).ok_or(HarnessError::Missing("a test-art move"))?;
    Ok(artwork.place_region(Some(lift), shift)?.artwork)
}

/// Two layers of strokes, the upper merged down into the lower, then strokes on the merged one.
fn merged(artwork: &Artwork, library: &BrushLibrary) -> Result<Artwork, HarnessError> {
    let mut artwork = layer(artwork)?;
    artwork = strokes_about(&artwork, library, (20, 14), 460.0)?;
    artwork = layer(&artwork)?;
    artwork = strokes_about(&artwork, library, (20, 15), 460.0)?;
    let merge = LayerCommand::MergeDown(artwork.active());
    artwork = artwork.apply(ArtworkCommand::Layer(merge))?.artwork;
    strokes_about(&artwork, library, (8, 16), 300.0)
}

/// A Multiply layer of strokes and above it a layer of strokes and a smudge, grouped, the group
/// at 85 %; the top layer active.
fn grouped(artwork: &Artwork, library: &BrushLibrary) -> Result<Artwork, HarnessError> {
    let mut artwork = layer(artwork)?;
    let multiply = LayerCommand::SetBlend {
        layer: artwork.active(),
        blend: BlendMode::Multiply,
    };
    artwork = artwork.apply(ArtworkCommand::Layer(multiply))?.artwork;
    artwork = strokes_about(&artwork, library, (20, 17), 440.0)?;
    artwork = layer(&artwork)?;
    artwork = strokes_about(&artwork, library, (20, 18), 440.0)?;
    let smudge =
        style(library, "soft-brush", 1.0, colour(5, 19)?, 0)?.with_tool(StrokeTool::Smudge);
    let hand = hand(5, 19, 300.0);
    artwork = stroke(&artwork, smudge, &hand.samples()?)?;
    let top = artwork.active();
    let group = ArtworkCommand::Group(GroupCommand::Create { layer: top });
    artwork = artwork.apply(group)?.artwork;
    let id = (artwork.layer(top).and_then(|layer| layer.props().group()))
        .ok_or(HarnessError::Missing("the test-art group"))?;
    let faded = GroupCommand::SetOpacity {
        group: id,
        opacity: 0.85,
    };
    Ok(artwork.apply(ArtworkCommand::Group(faded))?.artwork)
}

/// `count` strokes of the built-ins in turn starting within `radius` of the middle, their numbers
/// salted by `salt`.
fn strokes_about(
    artwork: &Artwork,
    library: &BrushLibrary,
    (count, salt): (u32, u64),
    radius: f32,
) -> Result<Artwork, HarnessError> {
    let mut artwork = artwork.clone();
    for k in 0..count {
        let key = u64::from(k) + salt * 1_000;
        let brush = BRUSHES[(k as usize + salt as usize) % BRUSHES.len()];
        let style = style(library, brush, 1.0, colour(key, salt)?, k)?;
        artwork = stroke(&artwork, style, &hand(key, salt, radius).samples()?)?;
    }
    Ok(artwork)
}

/// A hand's stroke of 120 to 260 samples starting within `radius` of the middle.
fn hand(key: u64, salt: u64, radius: f32) -> Hand {
    let (angle, reach) = (
        TAU * unit(key, salt + 1),
        radius * unit(key, salt + 2).sqrt(),
    );
    let [width, height] = SCREEN.map(|side| side as f32);
    Hand {
        start: [
            MIDDLE[0] + reach * angle.cos(),
            MIDDLE[1] + reach * angle.sin(),
        ],
        angle: TAU * unit(key, salt + 3),
        turn: (unit(key, salt + 4) - 0.5) * 0.03,
        count: 120 + (140.0 * unit(key, salt + 5)) as u32,
        spacing: 3.0 + 2.0 * unit(key, salt + 6),
        bounds: [60.0, 60.0, width - 60.0, height - 60.0],
    }
}
