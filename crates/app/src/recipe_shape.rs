//! A brush file's `[shape]` (`recipe.rs`): the edge's softness, and since S45 the stamps laid
//! along the path (DECISIONS D72): their shape, spacing, scatter, rotation and jitter. Without a
//! spacing the brush is the continuous round capsule, and no other stamp key may be set.

use openpaint_core::{
    MAX_SCATTER, MAX_SPACING, MIN_ASPECT, MIN_SPACING, Rotation, Shape, ShapeTexture, Stamp,
};

use crate::recipe::RecipeError;
use crate::recipe_section::{Section, missing, out_of_range};

/// The keys of `[shape]`.
const KEYS: [&str; 9] = [
    "softness",
    "stamp",
    "aspect",
    "spacing",
    "scatter",
    "rotation",
    "angle",
    "rotation_jitter",
    "size_jitter",
];

/// The edge's softness: 0 (hard) without `[shape]` or its `softness`.
pub(crate) fn softness(root: &Section<'_>) -> Result<f32, RecipeError> {
    let Some(shape) = root.section("shape")? else {
        return Ok(0.0);
    };
    shape.only(&KEYS)?;
    Ok(shape.number("softness")?.unwrap_or(0.0))
}

/// The stamps, or the continuous capsule without a spacing.
pub(crate) fn shape(root: &Section<'_>) -> Result<Shape, RecipeError> {
    let Some(section) = root.section("shape")? else {
        return Ok(Shape::CONTINUOUS);
    };
    section.only(&KEYS)?;
    let stamped = KEYS[1..]
        .iter()
        .any(|key| *key != "spacing" && section.table.contains_key(*key));
    let spacing = match section.number("spacing")? {
        Some(spacing) if spacing != 0.0 => spacing,
        spacing if stamped => {
            return Err(match spacing {
                None => missing(&section, "spacing"),
                Some(_) => out_of_range(&section, "spacing"),
            });
        }
        _ => return Ok(Shape::CONTINUOUS),
    };
    let stamp = stamp(&section)?;
    let aspect = section.number("aspect")?.unwrap_or(1.0);
    let rotation = rotation(&section)?;
    let value = |key: &str| Ok::<_, RecipeError>(section.number(key)?.unwrap_or(0.0));
    let (scatter, angle) = (value("scatter")?, value("angle")?);
    let (turn, size) = (value("rotation_jitter")?, value("size_jitter")?);
    let checks = [
        ("spacing", (MIN_SPACING..=MAX_SPACING).contains(&spacing)),
        ("aspect", aspect_fits(stamp, aspect)),
        ("scatter", (0.0..=MAX_SCATTER).contains(&scatter)),
        ("angle", (-180.0..=180.0).contains(&angle)),
        ("rotation_jitter", (0.0..=180.0).contains(&turn)),
        ("size_jitter", (0.0..=1.0).contains(&size)),
    ];
    if let Some((key, _)) = checks.iter().find(|(_, fits)| !fits) {
        return Err(out_of_range(&section, key));
    }
    Shape::stamped(
        (stamp, aspect),
        spacing,
        scatter,
        (rotation, angle, turn),
        size,
    )
    .map_err(|_| out_of_range(&section, "spacing"))
}

fn aspect_fits(stamp: Stamp, aspect: f32) -> bool {
    match stamp {
        Stamp::Disc => aspect == 1.0,
        _ => (MIN_ASPECT..=1.0).contains(&aspect),
    }
}

fn stamp(section: &Section<'_>) -> Result<Stamp, RecipeError> {
    Ok(match section.string("stamp")? {
        None | Some("disc") => Stamp::Disc,
        Some("square") => Stamp::Square,
        Some("ellipse") => Stamp::Ellipse,
        Some("chalk") => Stamp::Texture(ShapeTexture::Chalk),
        Some("grainy") => Stamp::Texture(ShapeTexture::Grainy),
        Some("splatter") => Stamp::Texture(ShapeTexture::Splatter),
        Some("rough-square") => Stamp::Texture(ShapeTexture::RoughSquare),
        Some(_) => return Err(out_of_range(section, "stamp")),
    })
}

fn rotation(section: &Section<'_>) -> Result<Rotation, RecipeError> {
    Ok(match section.string("rotation")? {
        None | Some("fixed") => Rotation::Fixed,
        Some("direction") => Rotation::Direction,
        Some("azimuth") => Rotation::Azimuth,
        Some(_) => return Err(out_of_range(section, "rotation")),
    })
}
