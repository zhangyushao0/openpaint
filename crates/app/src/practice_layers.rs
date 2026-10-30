//! A whole copy's two layers on its practice sheet (S95). V大: draw the division, lower its
//! opacity, then draw the line drawing over it. So the division's pens draw on a layer of their
//! own ("Division") and Draw on one above it ("Line drawing"), both over the sheet's first layer
//! that holds every other exercise; while Draw is chosen the division lies dimmed under the line
//! drawing, while compared it is hidden (the picture alone is laid over the copy). Which layer is
//! drawn on, how strongly the division shows and whether it shows follow the pen and Compare,
//! never steps of their own: the session amends the version shown (`History::amend`), so undo
//! walks the strokes alone. The layers are made, above the others, the first time the whole copy
//! shows with its own tools; they keep the names the app gave them (shown in the language).

use openpaint_core::{Artwork, ArtworkCommand, CoreError, LayerCommand, LayerId, LayerName};

use crate::practice_sheet::Layering;

/// The layers' names, as the app gives them (stored in English, shown in the language).
pub const DIVISION_LAYER: &str = "Division";
pub const LINES_LAYER: &str = "Line drawing";
/// How strongly the division shows under the line drawing while Draw is chosen.
pub const DIVISION_DIMMED: f32 = 0.3;

/// `artwork` laid as `layering` wants it: the same version when it already is.
///
/// # Errors
/// `LayerLimit` when the artwork has no room for the two layers.
pub(crate) fn layered(artwork: &Artwork, layering: Layering) -> Result<Artwork, CoreError> {
    let Layering::Copying { lines, hidden } = layering else {
        // Another exercise, or the end: the division shows again where there is one.
        return match named(artwork, DIVISION_LAYER) {
            Some(layer) => command(
                artwork,
                LayerCommand::SetVisible {
                    layer,
                    visible: true,
                },
            ),
            None => Ok(artwork.clone()),
        };
    };
    let (artwork, division) = made(artwork, DIVISION_LAYER)?;
    let (artwork, drawing) = made(&artwork, LINES_LAYER)?;
    let opacity = if lines { DIVISION_DIMMED } else { 1.0 };
    let artwork = command(
        &artwork,
        LayerCommand::SetOpacity {
            layer: division,
            opacity,
        },
    )?;
    let artwork = command(
        &artwork,
        LayerCommand::SetVisible {
            layer: division,
            visible: !hidden,
        },
    )?;
    command(
        &artwork,
        LayerCommand::Select(if lines { drawing } else { division }),
    )
}

/// The layer named `name`, if `artwork` has one.
fn named(artwork: &Artwork, name: &str) -> Option<LayerId> {
    (artwork.layers().iter())
        .find(|layer| layer.name().as_str() == name)
        .map(openpaint_core::Layer::id)
}

/// `artwork` with a layer named `name` (made above every other where it has none), and the layer.
fn made(artwork: &Artwork, name: &str) -> Result<(Artwork, LayerId), CoreError> {
    if let Some(layer) = named(artwork, name) {
        return Ok((artwork.clone(), layer));
    }
    let top = (artwork.layers().last())
        .map(openpaint_core::Layer::id)
        .ok_or(CoreError::InvalidValue {
            what: "a sheet's layers",
        })?;
    let artwork = command(artwork, LayerCommand::Select(top))?;
    let artwork = command(&artwork, LayerCommand::Create)?;
    let layer = artwork.active();
    let name = LayerName::new(name)?;
    let artwork = command(&artwork, LayerCommand::Rename { layer, name })?;
    Ok((artwork, layer))
}

fn command(artwork: &Artwork, command: LayerCommand) -> Result<Artwork, CoreError> {
    Ok(artwork.apply(ArtworkCommand::Layer(command))?.artwork)
}
