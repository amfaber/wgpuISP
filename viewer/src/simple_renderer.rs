use std::io::Read as _;

use gpwgpu::wgpu::{
    self, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutEntry, BufferDescriptor,
    BufferUsages, CommandEncoderDescriptor, Extent3d, FilterMode, ImageCopyTexture,
    ImageDataLayout, Origin3d, SamplerDescriptor, ShaderStages, TextureDescriptor, TextureUsages,
    TextureViewDescriptor,
};
use wgpu_isp::setup::State as ISPState;

use bevy::{
    asset::load_internal_asset,
    core_pipeline::core_2d::Transparent2d,
    ecs::system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
    prelude::*,
    render::{
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{BindGroup, Buffer, PipelineCache, SpecializedRenderPipelines},
        renderer::{RenderDevice, RenderQueue},
        view::{ViewUniformOffset, ViewUniforms, VisibleEntities},
        Extract, Render, RenderApp, RenderSet,
    },
    utils::FloatOrd,
};

use crate::my_sprite_pipeline::{
    SpritePipeline, SpritePipelineKey, SpriteVertex, QUAD_INDICES, QUAD_UVS, QUAD_VERTEX_POSITIONS,
    SPRITE_SHADER_HANDLE,
};

pub struct SimpleRendererPlugin;

impl Plugin for SimpleRendererPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SPRITE_SHADER_HANDLE,
            "my_sprite.wgsl",
            Shader::from_wgsl
        );
        app.insert_resource(Msaa::Off);

        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, extract_isp_image)
            .add_systems(Render, queue.in_set(RenderSet::Queue))
            .init_resource::<ViewUniformsResource>()
            .init_resource::<SpecializedRenderPipelines<SpritePipeline>>()
            .add_render_command::<Transparent2d, DrawIsp>();
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<SpritePipeline>();
    }
}

#[derive(Component)]
pub struct StateImage {
    pub state: ISPState<'static>,
    pub cpu_side_data: Option<Vec<f32>>,
    pub bind_group: BindGroup,
    pub vertex_buffer: Buffer,
}

#[derive(Component, Clone, Copy)]
pub struct ImageSettings {
    pub size: Vec2,
    pub anchor: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl StateImage {
    pub fn new(state: ISPState<'static>) -> Self {
        let layout =
            state
                .device
                .create_bind_group_layout(&gpwgpu::wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::FRAGMENT,
                            ty: gpwgpu::wgpu::BindingType::Texture {
                                sample_type: gpwgpu::wgpu::TextureSampleType::Float {
                                    filterable: true,
                                },
                                view_dimension: gpwgpu::wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::FRAGMENT,
                            ty: gpwgpu::wgpu::BindingType::Sampler(
                                gpwgpu::wgpu::SamplerBindingType::Filtering,
                            ),
                            count: None,
                        },
                    ],
                });
        let sampler = state.device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Linear,
            ..default()
        });

        // let view = {
        //     let mut file = std::fs::File::open("../left.rgba").unwrap();

        //     let width = 3840;
        //     let height = 2160;

        //     let mut buf = vec![0; width * height * 4];
        //     file.read_exact(&mut buf).unwrap();

        //     let size = Extent3d {
        //         width: width as u32,
        //         height: height as u32,
        //         depth_or_array_layers: 1,
        //     };
        //     let rgba_texture = state.device.create_texture(&TextureDescriptor {
        //         label: Some("rgba texture"),
        //         size,
        //         mip_level_count: 1,
        //         sample_count: 1,
        //         dimension: wgpu::TextureDimension::D2,
        //         format: wgpu::TextureFormat::Rgba8Unorm,
        //         usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        //         view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        //     });
        //     state.queue.write_texture(
        //         ImageCopyTexture {
        //             texture: &rgba_texture,
        //             mip_level: 0,
        //             origin: Origin3d { x: 0, y: 0, z: 0 },
        //             aspect: wgpu::TextureAspect::All,
        //         },
        //         &buf,
        //         ImageDataLayout {
        //             offset: 0,
        //             bytes_per_row: Some(width as u32 * 4),
        //             rows_per_image: Some(height as u32),
        //         },
        //         size,
        //     );
        //     dbg!("hello");

        //     state.queue.submit(None);
        //     // let mut encoder = state.device.create_command_encoder(&CommandEncoderDescriptor::default());

        //     // encoder.write

        //     rgba_texture.create_view(&TextureViewDescriptor {
        //         aspect: wgpu::TextureAspect::All,
        //         ..Default::default()
        //     })
        // };

        let view = state.texture.create_view(&TextureViewDescriptor::default());

        let bind_group = state.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: gpwgpu::wgpu::BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: gpwgpu::wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let vertex_buffer = state.device.create_buffer(&BufferDescriptor {
            label: None,
            size: 5 * 4 * 6,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            state,
            cpu_side_data: None,
            bind_group: bind_group.into(),
            vertex_buffer: vertex_buffer.into(),
        }
    }
}

fn extract_isp_image(
    mut commands: Commands,
    query: Extract<Query<(Entity, &GlobalTransform, &ImageSettings, &StateImage)>>,
) {
    for (
        entity,
        global,
        &image_settings,
        StateImage {
            state: _,
            cpu_side_data: _,
            bind_group,
            vertex_buffer,
        },
    ) in &query
    {
        commands.get_or_spawn(entity).insert(IspImage {
            entity,
            global: global.clone(),
            bind_group: bind_group.clone(),
            vertex_buffer: vertex_buffer.clone(),
            image_settings,
        });
    }
}

#[derive(Component)]
pub struct IspImage {
    pub entity: Entity,
    pub global: GlobalTransform,
    pub bind_group: BindGroup,
    pub vertex_buffer: Buffer,

    pub image_settings: ImageSettings,
}

#[derive(Resource, Default)]
pub struct ViewUniformsResource(Option<BindGroup>);

fn queue(
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    // mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    view_uniforms: Res<ViewUniforms>,
    mut view_uniforms_bindgroup: ResMut<ViewUniformsResource>,
    sprite_pipeline: Res<SpritePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SpritePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    mut views: Query<(
        &mut RenderPhase<Transparent2d>,
        &VisibleEntities,
        // &ExtractedView,
    )>,
    isp_images: Query<&IspImage>,
) {
    let draw_function = draw_functions.read().id::<DrawIsp>();

    let msaa_key = SpritePipelineKey::from_msaa_samples(1);

    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };

    let view_bind_group = render_device.create_bind_group(
        Some("sprite_view_bind_group"),
        &sprite_pipeline.view_layout,
        &[BindGroupEntry {
            binding: 0,
            resource: view_binding,
        }],
    );

    view_uniforms_bindgroup.0 = Some(view_bind_group);

    for (mut phase, visible_ents) in &mut views {
        let pipeline = pipelines.specialize(&pipeline_cache, &sprite_pipeline, msaa_key);
        for &visible in &visible_ents.entities {
            if let Ok(isp_image) = isp_images.get(visible) {
                let positions: [[f32; 3]; 4] = QUAD_VERTEX_POSITIONS.map(|quad_pos| {
                    isp_image
                        .global
                        .transform_point(
                            ((quad_pos - isp_image.image_settings.anchor)
                                * isp_image.image_settings.size)
                                .extend(0.),
                        )
                        .into()
                });

                let mut uvs = QUAD_UVS;
                if isp_image.image_settings.flip_x {
                    uvs = [uvs[1], uvs[0], uvs[3], uvs[2]];
                }
                if isp_image.image_settings.flip_y {
                    uvs = [uvs[3], uvs[2], uvs[1], uvs[0]];
                }

                let verts = QUAD_INDICES.map(|i| SpriteVertex {
                    position: positions[i],
                    uv: uvs[i].into(),
                });

                render_queue.write_buffer(
                    &isp_image.vertex_buffer,
                    0,
                    bytemuck::cast_slice(&verts),
                );

                phase.add(Transparent2d {
                    sort_key: FloatOrd(isp_image.global.translation().z),
                    entity: isp_image.entity,
                    pipeline,
                    draw_function,
                    batch_range: 0..1,
                    dynamic_offset: None,
                })
            }
        }
    }
}

type DrawIsp = (
    SetItemPipeline,
    SetIspViewBindGroup<0>,
    SetIspTextureBindGroup<1>,
    DrawIspCommand,
);

pub struct SetIspViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetIspViewBindGroup<I> {
    type Param = SRes<ViewUniformsResource>;
    type ViewQuery = Read<ViewUniformOffset>;
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: &'_ ViewUniformOffset,
        _entity: Option<()>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_group =
            unsafe { std::mem::transmute::<&Option<BindGroup>, &Option<BindGroup>>(&bind_group.0) };
        if let Some(bind_group) = bind_group {
            pass.set_bind_group(I, bind_group, &[view_uniform.offset]);
            return RenderCommandResult::Success;
        }
        return RenderCommandResult::Failure;
    }
}

pub struct SetIspTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetIspTextureBindGroup<I> {
    type Param = ();

    type ViewQuery = ();

    type ItemQuery = Read<IspImage>;

    fn render<'w>(
        _item: &P,
        _view: bevy::ecs::query::ROQueryItem<'w, Self::ViewQuery>,
        image: Option<bevy::ecs::query::ROQueryItem<'w, Self::ItemQuery>>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &image.unwrap().bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawIspCommand;

impl<P: PhaseItem> RenderCommand<P> for DrawIspCommand {
    type Param = ();

    type ViewQuery = ();

    type ItemQuery = Read<IspImage>;

    fn render<'w>(
        _item: &P,
        _view: bevy::ecs::query::ROQueryItem<'w, Self::ViewQuery>,
        isp_image: Option<bevy::ecs::query::ROQueryItem<'w, Self::ItemQuery>>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_vertex_buffer(0, isp_image.unwrap().vertex_buffer.slice(..));

        pass.draw(0..6, 0..1);
        RenderCommandResult::Success
    }
}
