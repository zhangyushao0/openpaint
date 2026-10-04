//! What the blit binds of the view tiers (S32): the second bind group of `blit.wgsl`, the
//! current tier's atlas and index, the previous tier's, and their scales. Without tiers (at or
//! below the canvas's own resolution) the core's group of one-pixel textures and scale 0.

use crate::core::Core;
use crate::view_atlas::Tier;

/// Bytes of the tiers' uniform: the current and the previous tier's scale (0: none), twice
/// padding.
const TIERS_BYTES: u64 = 16;

/// The tiers' bindings: atlas and index of the current tier, of the previous, the scales.
fn tier_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let texture = |binding, sample_type| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    let colour = wgpu::TextureSampleType::Float { filterable: false };
    let index = wgpu::TextureSampleType::Uint;
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("view tiers"),
        entries: &[
            texture(0, colour),
            texture(1, index),
            texture(2, colour),
            texture(3, index),
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(TIERS_BYTES),
                },
                count: None,
            },
        ],
    })
}

/// One-pixel stand-ins for a tier that is not there: a colour and an index texture.
struct Absent {
    colour: wgpu::Texture,
    index: wgpu::Texture,
}

impl Absent {
    fn new(device: &wgpu::Device) -> Self {
        let texture = |format, label| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        Self {
            colour: texture(wgpu::TextureFormat::Rgba8Unorm, "no tier"),
            index: texture(wgpu::TextureFormat::R32Uint, "no tier index"),
        }
    }
}

/// The tiers' bind group and uniform, rebuilt when a tier's textures change.
pub(crate) struct Shown {
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
    /// The textures the group binds: atlas and index of the current tier, then the previous.
    bound: [Option<(wgpu::Texture, wgpu::Texture)>; 2],
}

impl Shown {
    pub(crate) fn new(core: &Core) -> Self {
        let uniform = core.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view tiers"),
            size: TIERS_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = bind(core, &uniform, [None, None]);
        Self {
            uniform,
            group,
            bound: [None, None],
        }
    }

    /// Binds `tiers` (current, previous) if their textures changed and writes their scales.
    pub(crate) fn refresh(&mut self, core: &Core, tiers: [Option<&Tier>; 2]) {
        let textures =
            tiers.map(|tier| tier.map(|tier| (tier.atlas().clone(), tier.index().clone())));
        if textures != self.bound {
            self.group = bind(core, &self.uniform, tiers);
            self.bound = textures;
        }
        let [current, previous] = tiers.map(|tier| tier.map_or(0.0, |tier| tier.scale() as f32));
        let words = [current, previous, 0.0, 0.0];
        let bytes: Vec<u8> = words.iter().flat_map(|value| value.to_le_bytes()).collect();
        core.gpu.queue.write_buffer(&self.uniform, 0, &bytes);
    }

    pub(crate) fn group(&self) -> &wgpu::BindGroup {
        &self.group
    }
}

/// The tiers' bind group: their textures, or the absent ones.
fn bind(core: &Core, uniform: &wgpu::Buffer, tiers: [Option<&Tier>; 2]) -> wgpu::BindGroup {
    let shown = &core.tiers;
    group(
        &core.gpu.device,
        (&shown.layout, &shown.absent),
        uniform,
        tiers,
    )
}

/// The blit's knowledge of tiers in the core: their layout, the stand-ins, and the bind group
/// of no tiers at all.
pub(crate) struct TierBindings {
    pub(crate) layout: wgpu::BindGroupLayout,
    absent: Absent,
    pub(crate) none: wgpu::BindGroup,
}

impl TierBindings {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let layout = tier_layout(device);
        let absent = Absent::new(device);
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("no view tiers"),
            size: TIERS_BYTES,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: true,
        });
        // Mapped at creation: zeros, scale 0 for both.
        uniform.unmap();
        let none = group(device, (&layout, &absent), &uniform, [None, None]);
        Self {
            layout,
            absent,
            none,
        }
    }
}

fn group(
    device: &wgpu::Device,
    (layout, absent): (&wgpu::BindGroupLayout, &Absent),
    uniform: &wgpu::Buffer,
    tiers: [Option<&Tier>; 2],
) -> wgpu::BindGroup {
    let views: Vec<wgpu::TextureView> = tiers
        .iter()
        .flat_map(|tier| match tier {
            Some(tier) => [tier.atlas(), tier.index()],
            None => [&absent.colour, &absent.index],
        })
        .map(|texture| texture.create_view(&Default::default()))
        .collect();
    let mut entries: Vec<wgpu::BindGroupEntry<'_>> = (0..)
        .zip(&views)
        .map(|(binding, view)| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        })
        .collect();
    entries.push(wgpu::BindGroupEntry {
        binding: 4,
        resource: uniform.as_entire_binding(),
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("view tiers"),
        layout,
        entries: &entries,
    })
}
