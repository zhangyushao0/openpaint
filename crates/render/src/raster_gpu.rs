//! The raster core's pipelines (S57's probe, product code since S59; `raster.rs`): its shader
//! (raster.wgsl, raster_stamp.wgsl, raster_composite.wgsl, then the evaluator's own helpers,
//! spans, stamps, textures, deposits and fill edges, and the smudge engine's residual reading),
//! one pipeline per way a stroke lays into the atlas (the blend unit keeping the largest or
//! summing), per paint blend for a stroke's tiles and a fill onto the layer, one for patches, one
//! for smudges (S61), one per way a layer ends (`EndBlend`), and the finish per destination
//! format. Each pipeline's entry points reach only their own kind's code, so none pays for the
//! others' registers (D82).

use crate::error::RenderError;
use crate::gpu::{Gpu, TEN_BITS};
use crate::protocol::{Blend, prelude};
use crate::raster_plan::{Coverage, DrawKind, EndBlend};
use crate::raster_states::{
    Stage, blended, ended, floats, layouts, painted, pipeline_layout, summing,
};
use crate::readback::FORMAT;

const RASTER: &str = include_str!("shaders/raster.wgsl");
const RASTER_STAMP: &str = include_str!("shaders/raster_stamp.wgsl");
const RASTER_COMPOSITE: &str = include_str!("shaders/raster_composite.wgsl");
const EVALUATE: &str = include_str!("shaders/evaluate.wgsl");
const SEGMENT: &str = include_str!("shaders/segment.wgsl");
const STAMP: &str = include_str!("shaders/stamp.wgsl");
const STAMP_TEXTURE: &str = include_str!("shaders/stamp_texture.wgsl");
const DEPOSIT: &str = include_str!("shaders/deposit.wgsl");
const FILL_EDGE: &str = include_str!("shaders/fill_edge.wgsl");
const SMUDGE: &str = include_str!("shaders/smudge_composite.wgsl");

/// The helpers of evaluate.wgsl the raster core's stages call, taken from it whole, so they stay
/// its.
const HELPERS: [&str; 18] = [
    "fn word(",
    "fn stroke_paint(",
    "fn clip(",
    "fn paint_color(",
    "fn mask_bit(",
    "fn masked(",
    "fn patch_sample(",
    "fn patch_texel(",
    "fn over(",
    "fn mixed(",
    "fn composite(",
    "fn clipped(",
    "fn grain(",
    "const GRAIN_STEP",
    "fn hash(",
    "fn pcg(",
    "fn quantized(",
    "fn coded(",
];

/// The raster core's WGSL: the protocol prelude, its own stages, then the evaluator's functions
/// and the smudge engine's.
pub(crate) fn raster_source() -> Result<String, RenderError> {
    let mut source = prelude() + RASTER + RASTER_STAMP + RASTER_COMPOSITE;
    for head in HELPERS {
        source.push_str(item(EVALUATE, head).ok_or(RenderError::Invalid(
            "evaluate.wgsl lacks a helper the raster core takes",
        ))?);
    }
    Ok(source + SEGMENT + STAMP + STAMP_TEXTURE + DEPOSIT + FILL_EDGE + SMUDGE)
}

/// The declaration of `source` starting a line with `head`: a function through its closing
/// brace at the start of a line, a constant through its line.
fn item<'a>(source: &'a str, head: &str) -> Option<&'a str> {
    let start = source.find(&format!("\n{head}"))? + 1;
    let rest = &source[start..];
    let end = if head.starts_with("fn ") {
        rest.find("\n}\n")? + 3
    } else {
        rest.find('\n')? + 1
    };
    Some(&rest[..end])
}

/// The blends of a paint op, in `Blend`'s order.
const BLENDS: [Blend; 3] = [Blend::Over, Blend::Atop, Blend::Erase];

/// The ways a layer ends, in `EndBlend`'s order.
const ENDS: [EndBlend; 7] = [
    EndBlend::Over,
    EndBlend::Multiply,
    EndBlend::ScreenCanvas,
    EndBlend::Atop,
    EndBlend::ScreenChain,
    EndBlend::Base,
    EndBlend::Overlay,
];

pub(crate) struct RasterPipelines {
    /// The scene's buffers and the values (group 0); the textures a draw reads (group 1).
    pub(crate) layouts: [wgpu::BindGroupLayout; 2],
    /// By `Coverage`, in its order.
    coverage: [wgpu::RenderPipeline; 5],
    /// A stroke's tiles and a fill onto the layer, by `Blend`; a patch, over.
    strokes: [wgpu::RenderPipeline; 3],
    fills: [wgpu::RenderPipeline; 3],
    patch: wgpu::RenderPipeline,
    /// A smudge onto the layer or its mask from their copy, written whole.
    smudge: wgpu::RenderPipeline,
    /// By `EndBlend`, in its order.
    ends: [wgpu::RenderPipeline; 7],
    /// The finish into 8 bits, and into the display's format where it is ten bits.
    finish: wgpu::RenderPipeline,
    finish_ten: Option<wgpu::RenderPipeline>,
    /// The atlas's and the float targets' formats: 32-bit floats where the device blends them.
    pub(crate) atlas: wgpu::TextureFormat,
    pub(crate) layer: wgpu::TextureFormat,
    /// How many there are and how long making them took, ns (`Core::raster_pipelines`).
    pub(crate) count: usize,
    pub(crate) compile_ns: f64,
}

impl RasterPipelines {
    pub(crate) fn new(gpu: &Gpu) -> Result<Self, RenderError> {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raster core"),
            source: wgpu::ShaderSource::Wgsl(raster_source()?.into()),
        });
        let layouts = layouts(device);
        let scene = pipeline_layout(device, &layouts[..1]);
        let both = pipeline_layout(device, &layouts);
        let [atlas, layer] = floats(gpu);
        let make = |entries: [&str; 2], layout, target: wgpu::ColorTargetState| {
            let stage = Stage {
                module: &module,
                layout,
                entries,
            };
            stage.pipeline(device, target)
        };
        let coverage = Coverage::ALL.map(|coverage| {
            let (entries, operation) = coverage.stage();
            make(entries, &scene, blended(atlas, Some(summing(operation))))
        });
        let onto_layer = |entries: [&'static str; 2]| {
            BLENDS.map(|blend| make(entries, &both, blended(layer, Some(painted(blend)))))
        };
        let strokes = onto_layer(["composite_vertex", "composite_fragment"]);
        let fills = onto_layer(["rect_vertex", "fill_fragment"]);
        let patch = make(
            ["rect_vertex", "patch_fragment"],
            &both,
            blended(layer, Some(painted(Blend::Over))),
        );
        let smudge = make(
            ["smudge_vertex", "smudge_fragment"],
            &both,
            blended(layer, None),
        );
        let ends = ENDS.map(|end| {
            let (fragment, blend) = ended(end);
            make(["rect_vertex", fragment], &both, blended(layer, blend))
        });
        let finish = make(["finish_vertex", "finish_eight"], &both, FORMAT.into());
        let finish_ten = (gpu.display == TEN_BITS)
            .then(|| make(["finish_vertex", "finish_ten"], &both, TEN_BITS.into()));
        Ok(Self {
            layouts,
            coverage,
            strokes,
            fills,
            patch,
            smudge,
            ends,
            count: 5 + 3 + 3 + 1 + 1 + 7 + 1 + usize::from(finish_ten.is_some()),
            finish,
            finish_ten,
            atlas,
            layer,
            compile_ns: 0.0,
        })
    }

    pub(crate) fn coverage(&self, coverage: Coverage) -> &wgpu::RenderPipeline {
        &self.coverage[coverage as usize]
    }

    pub(crate) fn draw(&self, kind: DrawKind) -> &wgpu::RenderPipeline {
        match kind {
            DrawKind::Stroke(blend) => &self.strokes[blend as usize],
            DrawKind::Fill(blend) => &self.fills[blend as usize],
            DrawKind::Patch => &self.patch,
            DrawKind::Smudge => &self.smudge,
            DrawKind::End(end) => &self.ends[end as usize],
        }
    }

    /// The finish into `format` (8 bits, or the display's ten); `None` for another format.
    pub(crate) fn finish(&self, format: wgpu::TextureFormat) -> Option<&wgpu::RenderPipeline> {
        match format {
            FORMAT => Some(&self.finish),
            TEN_BITS => self.finish_ten.as_ref(),
            _ => None,
        }
    }
}

impl Coverage {
    const ALL: [Self; 5] = [
        Self::Cover,
        Self::Density,
        Self::Contact,
        Self::StampCover,
        Self::StampDensity,
    ];

    /// Its vertex and fragment entries, and how the blend unit combines what they lay.
    fn stage(self) -> ([&'static str; 2], wgpu::BlendOperation) {
        let (max, add) = (wgpu::BlendOperation::Max, wgpu::BlendOperation::Add);
        match self {
            Self::Cover => (["piece_vertex", "cover_fragment"], max),
            Self::Density => (["piece_vertex", "density_fragment"], add),
            Self::Contact => (["piece_vertex", "contact_fragment"], max),
            Self::StampCover => (["stamp_vertex", "stamp_fragment"], max),
            Self::StampDensity => (["stamp_vertex", "stamp_fragment"], add),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_raster_cores_shader_validates_with_the_evaluators_helpers() {
        let source = raster_source().expect("every helper is found");
        let module = wgpu::naga::front::wgsl::parse_str(&source).expect("the WGSL parses");
        let mut validator = wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        );
        validator.validate(&module).expect("the WGSL validates");
        assert_eq!(
            item("\nfn a(x) {\n  y\n}\nfn b", "fn a("),
            Some("fn a(x) {\n  y\n}\n")
        );
        assert_eq!(
            item("x\nconst K: u32 = 1u;\ny", "const K"),
            Some("const K: u32 = 1u;\n")
        );
    }
}
