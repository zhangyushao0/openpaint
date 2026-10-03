//! The text faces: the host hands the UI its system's sans (Roboto on Android) and its Chinese
//! face (Noto Sans CJK), read from the system's font files; the UI only owns the families.
//! Without them the built-in face shows Latin text and Chinese shows as boxes.

use egui::epaint::text::VariationCoords;
use egui::{FontData, FontDefinitions, FontFamily, FontTweak};

/// The weight of every text face, for faces with a weight axis: a plain regular, clearly
/// readable at arm's length.
const WEIGHT: f32 = 400.0;

/// One font file as the host read it: its bytes and the face in a collection (0 for a single
/// face).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFile {
    pub bytes: Vec<u8>,
    pub index: u32,
}

/// What the host hands over; either may be missing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextFonts {
    /// The system's sans for Latin text.
    pub latin: Option<FontFile>,
    /// A face with the Chinese characters.
    pub cjk: Option<FontFile>,
}

/// Puts the host's faces in front of the built-in ones: Latin first, then Chinese, then the
/// built-in faces for anything both lack.
pub(crate) fn install(ctx: &egui::Context, fonts: TextFonts) {
    let mut definitions = FontDefinitions::default();
    let mut front = Vec::new();
    for (name, file) in [("host-latin", fonts.latin), ("host-cjk", fonts.cjk)] {
        let Some(file) = file else {
            continue;
        };
        let mut data = FontData::from_owned(file.bytes);
        data.index = file.index;
        data.tweak = FontTweak {
            coords: VariationCoords::new([(b"wght", WEIGHT)]),
            ..FontTweak::default()
        };
        definitions.font_data.insert(name.into(), data.into());
        front.push(name.to_owned());
    }
    let proportional = definitions
        .families
        .entry(FontFamily::Proportional)
        .or_default();
    for (place, name) in front.iter().enumerate() {
        proportional.insert(place, name.clone());
    }
    // Monospace keeps its own face first; Chinese comes before the fallbacks.
    if let Some(cjk) = front.iter().find(|name| *name == "host-cjk") {
        let monospace = definitions
            .families
            .entry(FontFamily::Monospace)
            .or_default();
        monospace.insert(1.min(monospace.len()), cjk.clone());
    }
    ctx.set_fonts(definitions);
}
