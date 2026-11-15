//! The raster core's pipelines (`raster/`): its shader (raster.wgsl, raster_stamp.wgsl,
//! raster_composite.wgsl, raster_paint.wgsl, then spans, stamps, textures, deposits and fill
//! edges, the smudge engine's residual reading, adjust.wgsl and filter_composite.wgsl), one
//! pipeline per way a stroke lays into the atlas (the blend unit keeping the largest or summing),
//! per paint blend for a stroke's tiles and a fill onto the layer, one for patches, one for
//! smudges, one per way a layer ends (`EndBlend`), those for adjustments, filters, moves' ends and
//! a whole layer's cut, and the finish per destination format. Each pipeline's entry points reach only their own kind's code, so none pays for the
//! others' registers.

use crate::core::Core;
use crate::error::RenderError;
use crate::gpu::{Gpu, TEN_BITS};
use crate::protocol::{Blend, prelude};
use crate::raster_plan::{Coverage, DrawKind, EndBlend};
use crate::raster_states::{
    blended, cut, ended, floats, layouts, painted, pipeline, pipeline_layout, summing,
};
use crate::readback::FORMAT;

const RASTER: &str = include_str!("shaders/raster.wgsl");
const RASTER_STAMP: &str = include_str!("shaders/raster_stamp.wgsl");
const RASTER_COMPOSITE: &str = include_str!("shaders/raster_composite.wgsl");
const RASTER_PAINT: &str = include_str!("shaders/raster_paint.wgsl");
const SEGMENT: &str = include_str!("shaders/segment.wgsl");
const STAMP: &str = include_str!("shaders/stamp.wgsl");
const STAMP_TEXTURE: &str = include_str!("shaders/stamp_texture.wgsl");
const DEPOSIT: &str = include_str!("shaders/deposit.wgsl");
const FILL_EDGE: &str = include_str!("shaders/fill_edge.wgsl");
const SMUDGE: &str = include_str!("shaders/smudge_composite.wgsl");
const ADJUST: &str = include_str!("shaders/adjust.wgsl");
const FILTER: &str = include_str!("shaders/filter_composite.wgsl");

/// The raster core's WGSL: the protocol prelude, its stages, what they read of ops and words,
/// then the geometry of spans, stamps, deposits and fill edges, the smudge residuals' reading and
/// the adjustments' and filters' colour.
pub(crate) fn raster_source() -> String {
    prelude()
        + RASTER
        + RASTER_STAMP
        + RASTER_COMPOSITE
        + RASTER_PAINT
        + SEGMENT
        + STAMP
        + STAMP_TEXTURE
        + DEPOSIT
        + FILL_EDGE
        + SMUDGE
        + ADJUST
        + FILTER
}

/// The blends of a paint op, in `Blend`'s order.
const BLENDS: [Blend; 3] = [Blend::Over, Blend::Atop, Blend::Erase];

/// The ways a layer ends, in `EndBlend`'s order.
const ENDS: [EndBlend; 8] = [
    EndBlend::Over,
    EndBlend::Multiply,
    EndBlend::ScreenCanvas,
    EndBlend::Atop,
    EndBlend::ScreenChain,
    EndBlend::Base,
    EndBlend::Backdrop,
    EndBlend::Group,
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
    /// The kept coverage of the stroke being drawn read from its slots into the atlas, written
    /// whole.
    pub(crate) kept: wgpu::RenderPipeline,
    /// A smudge onto the layer or its mask from their copy, written whole; an adjustment
    /// and a filter likewise, a moved content laid over the level under it, a whole layer's cut
    /// in place.
    smudge: wgpu::RenderPipeline,
    records: [wgpu::RenderPipeline; 4],
    /// By `EndBlend`, in its order.
    ends: [wgpu::RenderPipeline; 8],
    /// The finish into 8 bits, and into the display's format where it is ten bits.
    finish: wgpu::RenderPipeline,
    finish_ten: Option<wgpu::RenderPipeline>,
    /// The atlas's and the float targets' formats: 32-bit floats where the device blends them.
    pub(crate) atlas: wgpu::TextureFormat,
    pub(crate) layer: wgpu::TextureFormat,
}

impl RasterPipelines {
    pub(crate) fn new(gpu: &Gpu) -> Result<Self, RenderError> {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raster core"),
            source: wgpu::ShaderSource::Wgsl(raster_source().into()),
        });
        let layouts = layouts(device);
        let scene = pipeline_layout(device, &layouts[..1]);
        let both = pipeline_layout(device, &layouts);
        let [atlas, layer] = floats(gpu);
        let make = |entries: [&str; 2], layout, target: wgpu::ColorTargetState| {
            pipeline(device, (&module, layout), entries, target)
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
        let [patch, smudge, kept] = [
            (
                ["rect_vertex", "patch_fragment"],
                blended(layer, Some(painted(Blend::Over))),
            ),
            (["arena_vertex", "smudge_fragment"], blended(layer, None)),
            (["kept_vertex", "kept_fragment"], blended(atlas, None)),
        ]
        .map(|(entries, target)| make(entries, &both, target));
        let (records, ends) = records(&|entries, target| make(entries, &both, target), layer);
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
            records,
            kept,
            ends,
            finish,
            finish_ten,
            atlas,
            layer,
        })
    }

    /// How many pipelines it holds.
    fn count(&self) -> usize {
        let single = [&self.patch, &self.kept, &self.smudge, &self.finish].len();
        self.coverage.len()
            + self.strokes.len()
            + self.fills.len()
            + self.records.len()
            + self.ends.len()
            + single
            + usize::from(self.finish_ten.is_some())
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
            DrawKind::Adjust => &self.records[0],
            DrawKind::Filter => &self.records[1],
            DrawKind::Moved => &self.records[2],
            DrawKind::Cut => &self.records[3],
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

/// The pipelines onto the layer's float targets (in `layer`) that `make` makes from their entries
/// and target: the records (an adjustment and a filter from the copy, a moved content over the
/// level under it, a whole layer's cut), then the layer ends by `EndBlend`.
fn records(
    make: &impl Fn([&'static str; 2], wgpu::ColorTargetState) -> wgpu::RenderPipeline,
    layer: wgpu::TextureFormat,
) -> ([wgpu::RenderPipeline; 4], [wgpu::RenderPipeline; 8]) {
    let records = [
        (["rect_vertex", "adjust_fragment"], None),
        (["arena_vertex", "filter_fragment"], None),
        (
            ["rect_vertex", "moved_fragment"],
            Some(painted(Blend::Over)),
        ),
        (["rect_vertex", "cut_fragment"], Some(cut())),
    ]
    .map(|(entries, blend)| make(entries, blended(layer, blend)));
    let ends = ENDS.map(|end| {
        let (fragment, blend) = ended(end);
        make(["rect_vertex", fragment], blended(layer, blend))
    });
    (records, ends)
}

impl Core {
    /// The raster core's pipelines, made by the first caller (how long that took is logged), any
    /// other waiting until they are: the runtime makes them on its raster worker as the device is
    /// made, so the screen's frames never wait for them (`raster_made`).
    pub(crate) fn raster_pipelines(&self) -> Result<&RasterPipelines, RenderError> {
        if let Some(pipelines) = self.raster.get() {
            return Ok(pipelines);
        }
        let _one = self
            .making
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(pipelines) = self.raster.get() {
            return Ok(pipelines);
        }
        let started = std::time::Instant::now();
        let made = RasterPipelines::new(&self.gpu)?;
        log::info!(
            "raster core: {} pipelines made in {:.0} ms ({:?} atlas, {:?} targets)",
            made.count(),
            started.elapsed().as_secs_f64() * 1e3,
            made.atlas,
            made.layer
        );
        Ok(self.raster.get_or_init(|| made))
    }

    /// The raster core's pipelines when they are made, without waiting.
    pub(crate) fn raster_made(&self) -> Option<&RasterPipelines> {
        self.raster.get()
    }

    /// Makes the raster core's pipelines now unless they are made (the runtime's raster
    /// worker, as the device is made, so the first zoom finds them).
    ///
    /// # Errors
    /// As the GPU: a shader the device refuses.
    pub fn make_raster_pipelines(&self) -> Result<(), RenderError> {
        self.raster_pipelines().map(drop)
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
    fn the_raster_cores_shader_validates() {
        let source = raster_source();
        let module = wgpu::naga::front::wgsl::parse_str(&source).expect("the WGSL parses");
        let mut validator = wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        );
        validator.validate(&module).expect("the WGSL validates");
    }
}
