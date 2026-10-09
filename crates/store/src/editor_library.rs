//! The brush library at the toolbox record's end (schema 21, S46): a count (u32), then per
//! brush in picker order its key (u32 length + UTF-8) and a kind (u8: 0 a built-in as its file
//! has it, 1 a whole brush); a whole brush then holds its name and its source's key (empty for a
//! built-in; strings as the key), its size range's least, first and largest diameter, its first
//! opacity and StreamLine (f32 each) and its recipe as a stroke record holds it (`strokes.rs`).
//! A count of 0 is the built-ins as their files have them, which schemas 3 to 20 held.

use openpaint_app::{BrushParts, PortError, StoredBrush};

use crate::codec::{Reader, Writer};
use crate::strokes::{read_brush_recipe, write_whole_recipe};

/// Bytes of a brush entry with an empty key, by key alone.
const ENTRY_BYTES: usize = 4 + 1;

pub(crate) fn write_library(writer: &mut Writer, library: &[StoredBrush]) {
    writer.len(library.len());
    for entry in library {
        writer.str(&entry.key);
        let Some(brush) = &entry.brush else {
            writer.u8(0);
            continue;
        };
        writer.u8(1);
        writer.str(&brush.name);
        writer.str(&brush.source);
        for value in brush.size {
            writer.f32(value);
        }
        writer.f32(brush.opacity);
        writer.f32(brush.streamline);
        write_whole_recipe(writer, &brush.recipe);
    }
}

pub(crate) fn read_library(reader: &mut Reader<'_>) -> Result<Vec<StoredBrush>, PortError> {
    let count = reader.count(ENTRY_BYTES)?;
    let mut library = Vec::with_capacity(count);
    for _ in 0..count {
        let key = reader.str()?;
        let brush = match reader.u8()? {
            0 => None,
            1 => Some(BrushParts {
                name: reader.str()?,
                source: reader.str()?,
                size: [reader.f32()?, reader.f32()?, reader.f32()?],
                opacity: reader.f32()?,
                streamline: reader.f32()?,
                recipe: read_brush_recipe(reader)?,
            }),
            _ => return Err(reader.corrupt()),
        };
        library.push(StoredBrush { key, brush });
    }
    Ok(library)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_core::{BrushRecipe, Rotation, ShapeParts, ShapeTexture, Stamp};

    fn chalk() -> BrushParts {
        let shape = ShapeParts {
            stamp: Stamp::Texture(ShapeTexture::Chalk),
            spacing: 0.4,
            rotation: Rotation::Direction,
            ..ShapeParts::CONTINUOUS
        };
        let mut recipe = BrushRecipe::plain().to_parts();
        recipe.shape = shape;
        recipe.taper = [5.0, 3.0, 0.1];
        BrushParts {
            name: "Inker copy ✒".into(),
            source: "inker".into(),
            size: [1.0, 12.0, 80.0],
            opacity: 1.0,
            streamline: 0.25,
            recipe,
        }
    }

    #[test]
    fn the_library_round_trips_and_a_cut_or_unknown_entry_is_corrupt() {
        // A recipe the engine refuses is still read: the store never judges recipes, the
        // toolbox falls back to the built-in it comes from (the app's tests).
        let mut refused = chalk();
        refused.recipe.taper = [-1.0, 0.0, 0.0];
        let library = vec![
            StoredBrush {
                key: "pencil".into(),
                brush: None,
            },
            StoredBrush {
                key: "copy-1".into(),
                brush: Some(chalk()),
            },
            StoredBrush {
                key: "copy-2".into(),
                brush: Some(refused),
            },
        ];
        let mut writer = Writer::default();
        write_library(&mut writer, &library);
        let bytes = writer.finish();
        let mut reader = Reader::new(&bytes, "editor state");
        assert_eq!(read_library(&mut reader), Ok(library));
        assert_eq!(reader.end(), Ok(()));
        let corrupt = Err(PortError::Corrupt {
            what: "editor state",
        });
        let cut = &bytes[..bytes.len() - 1];
        assert_eq!(read_library(&mut Reader::new(cut, "editor state")), corrupt);
        // The first entry's kind: after the count and "pencil".
        let mut unknown = bytes.clone();
        unknown[4 + 4 + 6] = 2;
        assert_eq!(
            read_library(&mut Reader::new(&unknown, "editor state")),
            corrupt
        );
        let mut empty = Writer::default();
        write_library(&mut empty, &[]);
        assert_eq!(empty.finish(), [0; 4], "the built-ins: a count of 0");
    }
}
