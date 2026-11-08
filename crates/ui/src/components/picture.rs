//! Pictures the renderer and the store make (gallery cards, brush rows, layer rows, the brush
//! strip, the reference image) as egui textures, sampled linearly when drawn larger or smaller.

/// How a picture's colour channels hold its alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alpha {
    /// Colour as it shows, alpha beside it.
    Straight,
    /// Colour already multiplied by alpha (the renderer's layer pictures).
    Premultiplied,
}

/// `pixels`, 8-bit RGBA rows top first of `size`, as a texture named `name`.
pub fn picture_texture(
    ctx: &egui::Context,
    name: impl Into<String>,
    (size, pixels, alpha): ([u32; 2], &[u8], Alpha),
) -> egui::TextureHandle {
    let size = size.map(|side| side as usize);
    let image = match alpha {
        Alpha::Straight => egui::ColorImage::from_rgba_unmultiplied(size, pixels),
        Alpha::Premultiplied => egui::ColorImage::from_rgba_premultiplied(size, pixels),
    };
    ctx.load_texture(name, image, egui::TextureOptions::LINEAR)
}
