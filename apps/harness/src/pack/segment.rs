//! The salient-object segmentation the builder outlines pictures with (S81): U²-Net (Qin et al.,
//! 2020, <https://github.com/xuebinqin/U-2-Net>, Apache-2.0, weights included), as the ONNX file
//! rembg publishes (<https://github.com/danielgatis/rembg>, MIT), run by tract in pure Rust. The
//! model is downloaded once into the builder's cache (`models/`), never committed, and checked
//! by its size and hash before every use; its matte becomes an outline in the practice crate
//! (`outline_of_matte`), so the model never reaches the app.
//!
//! The picture goes in as U²-Net was trained and rembg feeds it: laid on white where it is
//! see-through, stretched to 320 x 320 (Lanczos), scaled by its brightest value, less ImageNet's
//! mean and over its deviation per channel; the first output (the fused map, a sigmoid) is the
//! matte, at 320 x 320 over the whole picture. tract runs it on one thread, so the same machine
//! gives the same matte to the bit, and the builder the same pack (two builds compared, S81).

use image::imageops::{self, FilterType};
use image::{Rgb, RgbImage};
use openpaint_practice::Matte;
use tract_onnx::prelude::*;

use super::fetch::{Fetcher, fnv_bytes};
use crate::error::HarnessError;

/// A segmentation model as the builder fetches and feeds it.
struct Model {
    /// Its file in the cache and where it is downloaded from.
    file: &'static str,
    url: &'static str,
    /// Its length in bytes and FNV-1a hash (`fnv_bytes`).
    bytes: usize,
    hash: u64,
    /// The square side it takes, and the per-channel mean and deviation it was trained with.
    side: u32,
    mean: [f32; 3],
    deviation: [f32; 3],
}

/// U²-Net's full model: rembg's `u2net.onnx`, MD5 60024c5c889badc19c04ad937298a77b as rembg
/// states it, SHA-256 8d10d2f3bb75ae3b6d527c77944fc5e7dcd94b29809d47a739a7a728a912b491.
const U2NET: Model = Model {
    file: "models/u2net.onnx",
    url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx",
    bytes: 175_997_641,
    hash: 0x8d1b_a36b_8735_7ea7,
    side: 320,
    mean: [0.485, 0.456, 0.406],
    deviation: [0.229, 0.224, 0.225],
};

/// The model loaded and ready to run.
pub struct Segmenter {
    model: &'static Model,
    plan: std::sync::Arc<TypedRunnableModel>,
}

impl Segmenter {
    /// U²-Net from the cache (downloaded on a miss), checked and optimised for its input.
    ///
    /// # Errors
    /// `Fetch` when it cannot be downloaded, is not the file expected, or does not load.
    pub fn load(fetcher: &Fetcher) -> Result<Self, HarnessError> {
        let model = &U2NET;
        let failed = |detail: String| HarnessError::Fetch {
            url: model.url.to_owned(),
            detail,
        };
        let bytes = fetcher.get(model.file, model.url)?;
        let hash = fnv_bytes(&bytes);
        if bytes.len() != model.bytes || hash != model.hash {
            return Err(failed(format!(
                "{} bytes, hash {hash:016x}: not the model expected ({} bytes, {:016x})",
                bytes.len(),
                model.bytes,
                model.hash
            )));
        }
        let side = model.side as usize;
        let plan = tract_onnx::onnx()
            .model_for_read(&mut bytes.as_slice())
            .and_then(|graph| graph.with_input_fact(0, f32::fact([1, 3, side, side]).into()))
            .and_then(|graph| graph.into_optimized())
            .and_then(|graph| graph.into_runnable())
            .map_err(|error| failed(format!("the model does not load: {error:#}")))?;
        Ok(Self { model, plan })
    }

    /// The matte of the picture `rgba` (8-bit sRGB, straight alpha, `size` pixels).
    ///
    /// # Errors
    /// `Missing` when the pixels are not a picture of `size` or the model does not run.
    pub fn matte(&self, size: [u32; 2], rgba: &[u8]) -> Result<Matte, HarnessError> {
        let model = self.model;
        let side = model.side;
        let on_white: Vec<u8> = (rgba.as_chunks::<4>().0.iter())
            .flat_map(|p| {
                let alpha = u32::from(p[3]);
                [0, 1, 2]
                    .map(|c| ((u32::from(p[c]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8)
            })
            .collect();
        let picture = RgbImage::from_raw(size[0], size[1], on_white).ok_or(
            HarnessError::Missing("a picture's pixels are not of its size"),
        )?;
        let input = imageops::resize(&picture, side, side, FilterType::Lanczos3);
        let brightest = f32::from(input.pixels().flat_map(|Rgb(p)| *p).max().unwrap_or(0)).max(1.0);
        let side = side as usize;
        let tensor: Tensor =
            tract_ndarray::Array4::from_shape_fn((1, 3, side, side), |(_, c, y, x)| {
                let value = f32::from(input.get_pixel(x as u32, y as u32).0[c]) / brightest;
                (value - model.mean[c]) / model.deviation[c]
            })
            .into();
        let outputs = self
            .plan
            .run(tvec!(tensor.into()))
            .map_err(|_| HarnessError::Missing("the segmentation model did not run"))?;
        let map = outputs[0]
            .to_plain_array_view::<f32>()
            .map_err(|_| HarnessError::Missing("the segmentation model gave no map"))?;
        let values: Vec<f32> = map.iter().map(|v| v.clamp(0.0, 1.0)).collect();
        if values.len() != side * side {
            return Err(HarnessError::Missing(
                "the segmentation model's map is not its side",
            ));
        }
        Ok(Matte {
            size: [model.side, model.side],
            values,
        })
    }
}
