//! A whole copy's layers on its practice sheet (S95, S96). V大: draw the division, lower its
//! opacity, then draw the line drawing over it, or paint the colour blocks. So the division's pens
//! draw on a layer of their own ("Division"), Draw on one above it ("Line drawing"), and the
//! painting pens on one under it ("Colour blocks", made the first time one is chosen), all over
//! the sheet's first layer that holds every other exercise; while Draw or a painting pen is
//! chosen the division lies dimmed (over the blocks it stays a guide), while compared it is
//! hidden (the picture alone is laid over the copy). Which layer is drawn on, how strongly the
//! division shows and whether it shows follow the pen and Compare, never steps of their own: the
//! session amends the version shown (`History::amend`), so undo walks the strokes alone. The
//! division and line drawing are made, above the others, the first time the whole copy shows with
//! its own tools; the layers keep the names the app gave them (shown in the language).

use openpaint_core::{Artwork, ArtworkCommand, CoreError, LayerCommand, LayerId, LayerName};

use crate::sheet::{CopyLayer, Layering};

/// The layers' names, as the app gives them (stored in English, shown in the language).
pub const DIVISION_LAYER: &str = "Division";
pub const LINES_LAYER: &str = "Line drawing";
pub const BLOCKS_LAYER: &str = "Colour blocks";
/// How strongly the division shows over the colour blocks or under the line drawing while Draw
/// or a painting pen is chosen.
pub const DIVISION_DIMMED: f32 = 0.3;

/// `artwork` laid as `layering` wants it: the same version when it already is.
///
/// # Errors
/// `LayerLimit` when the artwork has no room for the layers.
pub(crate) fn layered(artwork: &Artwork, layering: Layering) -> Result<Artwork, CoreError> {
    let Layering::Copying { on, hidden } = layering else {
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
    let (artwork, division) = made(artwork, DIVISION_LAYER, None)?;
    let (artwork, drawing) = made(&artwork, LINES_LAYER, None)?;
    let (artwork, drawn_on) = match on {
        CopyLayer::Division => (artwork, division),
        CopyLayer::Lines => (artwork, drawing),
        CopyLayer::Blocks => made(&artwork, BLOCKS_LAYER, Some(division))?,
    };
    let opacity = if on == CopyLayer::Division {
        1.0
    } else {
        DIVISION_DIMMED
    };
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
    command(&artwork, LayerCommand::Select(drawn_on))
}

/// The layer named `name`, if `artwork` has one.
fn named(artwork: &Artwork, name: &str) -> Option<LayerId> {
    (artwork.layers().iter())
        .find(|layer| layer.name().as_str() == name)
        .map(openpaint_core::Layer::id)
}

/// `artwork` with a layer named `name` (made where it has none: right under `under`, else above
/// every other), and the layer.
fn made(
    artwork: &Artwork,
    name: &str,
    under: Option<LayerId>,
) -> Result<(Artwork, LayerId), CoreError> {
    if let Some(layer) = named(artwork, name) {
        return Ok((artwork.clone(), layer));
    }
    let layers = artwork.layers();
    let below = match under {
        Some(above) => {
            let at = layers.iter().position(|layer| layer.id() == above);
            at.and_then(|at| at.checked_sub(1))
                .map(|at| layers[at].id())
        }
        None => layers.last().map(openpaint_core::Layer::id),
    };
    let below = below.ok_or(CoreError::InvalidValue {
        what: "a sheet's layers",
    })?;
    let artwork = command(artwork, LayerCommand::Select(below))?;
    let artwork = command(&artwork, LayerCommand::Create)?;
    let layer = artwork.active();
    let name = LayerName::new(name)?;
    let artwork = command(&artwork, LayerCommand::Rename { layer, name })?;
    Ok((artwork, layer))
}

fn command(artwork: &Artwork, command: LayerCommand) -> Result<Artwork, CoreError> {
    Ok(artwork.apply(ArtworkCommand::Layer(command))?.artwork)
}
