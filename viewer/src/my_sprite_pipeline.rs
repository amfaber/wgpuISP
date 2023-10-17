use bevy::{
    ecs::system::SystemState,
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayout, FragmentState, RenderPipelineDescriptor, ShaderType,
            SpecializedRenderPipeline, VertexBufferLayout, VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{
            BevyDefault, DefaultImageSampler, GpuImage, ImageSampler, TextureFormatPixelInfo,
        },
        view::{ViewTarget, ViewUniform},
    },
    sprite::SPRITE_SHADER_HANDLE, core::{Zeroable, Pod},
};
use gpwgpu::wgpu::{
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState, BufferBindingType,
    ColorTargetState, ColorWrites, FrontFace, ImageCopyTexture, ImageDataLayout, MultisampleState,
    Origin3d, PolygonMode, PrimitiveState, PrimitiveTopology, SamplerBindingType, ShaderStages,
    TextureAspect, TextureFormat, TextureSampleType, TextureViewDescriptor, TextureViewDimension,
    VertexFormat, VertexStepMode,
};

pub const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

pub const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::new(-0.5, -0.5),
    Vec2::new(0.5, -0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(-0.5, 0.5),
];

pub const QUAD_UVS: [Vec2; 4] = [
    Vec2::new(0., 1.),
    Vec2::new(1., 1.),
    Vec2::new(1., 0.),
    Vec2::new(0., 0.),
];

#[derive(Resource)]
pub struct SpritePipeline {
    pub view_layout: BindGroupLayout,
    pub material_layout: BindGroupLayout,
    pub dummy_white_gpu_image: GpuImage,
}

impl FromWorld for SpritePipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<RenderDevice>,
            Res<DefaultImageSampler>,
            Res<RenderQueue>,
        )> = SystemState::new(world);
        let (render_device, default_sampler, render_queue) = system_state.get_mut(world);

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(ViewUniform::min_size()),
                },
                count: None,
            }],
            label: Some("sprite_view_layout"),
        });

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("sprite_material_layout"),
        });
        let dummy_white_gpu_image = {
            let image = Image::default();
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = match image.sampler_descriptor {
                ImageSampler::Default => (**default_sampler).clone(),
                ImageSampler::Descriptor(descriptor) => render_device.create_sampler(&descriptor),
            };

            let format_size = image.texture_descriptor.format.pixel_size();
            render_queue.write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &image.data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(image.texture_descriptor.size.width * format_size as u32),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );
            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                texture_format: image.texture_descriptor.format,
                sampler,
                size: Vec2::new(
                    image.texture_descriptor.size.width as f32,
                    image.texture_descriptor.size.height as f32,
                ),
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };

        SpritePipeline {
            view_layout,
            material_layout,
            dummy_white_gpu_image,
        }
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    pub struct SpritePipelineKey: u32 {
        const NONE                              = 0;
        const COLORED                           = (1 << 0);
        const HDR                               = (1 << 1);
        const TONEMAP_IN_SHADER                 = (1 << 2);
        const DEBAND_DITHER                     = (1 << 3);
        const MSAA_RESERVED_BITS                = Self::MSAA_MASK_BITS << Self::MSAA_SHIFT_BITS;
        const TONEMAP_METHOD_RESERVED_BITS      = Self::TONEMAP_METHOD_MASK_BITS << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_NONE               = 0 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_REINHARD           = 1 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_REINHARD_LUMINANCE = 2 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_ACES_FITTED        = 3 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_AGX                = 4 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM = 5 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_TONY_MC_MAPFACE    = 6 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_BLENDER_FILMIC     = 7 << Self::TONEMAP_METHOD_SHIFT_BITS;
    }
}

impl SpritePipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111;
    const MSAA_SHIFT_BITS: u32 = 32 - Self::MSAA_MASK_BITS.count_ones();
    const TONEMAP_METHOD_MASK_BITS: u32 = 0b111;
    const TONEMAP_METHOD_SHIFT_BITS: u32 =
        Self::MSAA_SHIFT_BITS - Self::TONEMAP_METHOD_MASK_BITS.count_ones();

    #[inline]
    pub const fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits_retain(msaa_bits)
    }

    #[inline]
    pub const fn msaa_samples(&self) -> u32 {
        1 << ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS)
    }

    #[inline]
    pub const fn from_colored(colored: bool) -> Self {
        if colored {
            SpritePipelineKey::COLORED
        } else {
            SpritePipelineKey::NONE
        }
    }

    #[inline]
    pub const fn from_hdr(hdr: bool) -> Self {
        if hdr {
            SpritePipelineKey::HDR
        } else {
            SpritePipelineKey::NONE
        }
    }
}

impl SpecializedRenderPipeline for SpritePipeline {
    type Key = SpritePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut formats = vec![
            // position
            VertexFormat::Float32x3,
            // uv
            VertexFormat::Float32x2,
        ];

        if key.contains(SpritePipelineKey::COLORED) {
            // color
            formats.push(VertexFormat::Float32x4);
        }

        let vertex_layout =
            VertexBufferLayout::from_vertex_formats(VertexStepMode::Vertex, formats);

        let mut shader_defs = Vec::new();
        if key.contains(SpritePipelineKey::COLORED) {
            shader_defs.push("COLORED".into());
        }

        if key.contains(SpritePipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());

            let method = key.intersection(SpritePipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == SpritePipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_AGX {
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
            {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_BLENDER_FILMIC {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            }

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(SpritePipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        let format = match key.contains(SpritePipelineKey::HDR) {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: SPRITE_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
            },
            fragment: Some(FragmentState {
                shader: SPRITE_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.view_layout.clone(), self.material_layout.clone()],
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("sprite_pipeline".into()),
            push_constant_ranges: Vec::new(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct SpriteVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

