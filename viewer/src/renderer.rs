use std::{
    borrow::Cow,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use bevy::{
    asset::load_internal_asset,
    core_pipeline::core_3d,
    ecs::{
        query::{QueryItem, ROQueryItem},
        system::{
            lifetimeless::{Read, SRes, SResMut},
            SystemParamItem,
        },
    },
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{
        self,
        camera::ExtractedCamera,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::MeshVertexBufferLayout,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner,
        },
        render_phase::{
            AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions,
            PhaseItem, RenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::{
            CachedTexture, FallbackImage, ImageSampler, TextureCache,
            TextureFormatPixelInfo,
        },
        view::{ExtractedView, ViewDepthTexture, ViewTarget, VisibleEntities},
        Extract, Render, RenderApp, RenderSet,
    },
    utils::{FloatOrd, HashMap},
};

// pub const VOLUME_RENDER_HANDLE: HandleUntyped =
//     HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2645551194423108407);


pub struct ISPRenderPlugin;

impl Plugin for ISPRenderPlugin {
    fn build(&self, app: &mut App) {
        // load_internal_asset!(
        //     app,
        //     VOLUME_RENDER_HANDLE,
        //     "shaders/volume_render.wgsl",
        //     Shader::from_wgsl
        // );

        app.add_plugins((
            ExtractComponentPlugin::<Handle<ISPRenderAsset>>::default(),
            RenderAssetPlugin::<ISPRenderAsset>::default(),
        ))
        .insert_resource(Msaa::Off)
        .add_asset::<ISPRenderAsset>();

        app.sub_app_mut(RenderApp)
            .init_resource::<DrawFunctions<ISPPhaseItem>>()
            .add_render_command::<ISPPhaseItem, DrawISP>()
            .init_resource::<SpecializedMeshPipelines<ISPPipeline>>()
            .init_resource::<RenderAssets<ISPRenderAsset>>()
            .add_systems(ExtractSchedule, extract_camera_isp_phase)
            .add_systems(
                Render,
                (
                    queue.in_set(RenderSet::Queue),
                    // prepare_render_textures
                        // .in_set(RenderSet::Prepare)
                        // .after(render::view::prepare_windows),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<ISPNode>>(
                core_3d::graph::NAME,
                ISPNode::NAME,
            )
            .add_render_graph_edges(
                core_3d::graph::NAME,
                &[
                    core_3d::graph::node::MAIN_TRANSPARENT_PASS,
                    ISPNode::NAME,
                    core_3d::graph::node::END_MAIN_PASS,
                ],
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<ISPPipeline>();
    }
}

#[derive(Debug, Clone, ShaderType, Default, TypePath, TypeUuid)]
#[uuid = "6ea266a6-6cf3-53a4-9287-1dabf5c17d6f"]
pub struct RaycastInput {
    pub bounding_box: [Vec3; 2],
    pub cmap_bounds: Vec2,
}

enum UpdatedSignal {
    MainWorld(AtomicBool),
    RenderWorld(bool),
}

pub struct Updated<T: Clone> {
    pub data: T,
    updated: UpdatedSignal,
}

impl<T: Clone> Updated<T> {
    fn new(data: T, updated: bool) -> Self {
        Self {
            data,
            updated: UpdatedSignal::MainWorld(AtomicBool::new(updated)),
        }
    }

    fn take_clone(&self) -> Self {
        let UpdatedSignal::MainWorld(atomic) = &self.updated else {
            unreachable!("take_clone should only be called on updates from the
                main world");
        };
        let updated = atomic.swap(false, Ordering::SeqCst);
        Self {
            data: self.data.clone(),
            updated: UpdatedSignal::RenderWorld(updated),
        }
    }
}

#[derive(AsBindGroup, TypePath, TypeUuid)]
#[uuid = "6ea266a6-6cf3-53a4-9986-1d7bf5c12396"]
pub struct ISPRenderAsset {
}

impl RenderAsset for ISPRenderAsset {
    type ExtractedAsset = Self;

    type PreparedAsset = PreparedISP;

    type Param = ();

    fn extract_asset(&self) -> Self::ExtractedAsset {
		todo!()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
		todo!()
	}
}

#[allow(unused)]
#[derive(Component)]
pub struct PreparedISP {
    pub bindings: Vec<OwnedBindingResource>,
    pub bind_group: BindGroup,
    pub key: <ISPRenderAsset as AsBindGroup>::Data,
}

#[derive(Default)]
struct ISPNode;

impl ISPNode {
    const NAME: &str = "isp";
}

impl ViewNode for ISPNode {
    type ViewQuery = ();
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
		todo!()
	}
}

pub struct ISPPhaseItem {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for ISPPhaseItem {
    type SortKey = FloatOrd;

    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        items.sort_by_key(|item| FloatOrd(item.distance));
    }
}

impl CachedRenderPipelinePhaseItem for ISPPhaseItem {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_camera_isp_phase(
    mut commands: Commands,
    cameras_3d: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
) {
    for (entity, camera) in &cameras_3d {
        if camera.is_active {
            commands
                .get_or_spawn(entity)
                .insert((
                    RenderPhase::<ISPPhaseItem>::default(),
                    // NeedsCompositeBindGroup,
                ));
        }
    }
}

fn queue(
    volume_3d_draw_functions: Res<DrawFunctions<ISPPhaseItem>>,
    volume_pipeline: Res<ISPPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<ISPPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<Mesh>>,
    material_meshes: Query<(Entity, &MeshUniform, &Handle<Mesh>), With<Handle<ISPRenderAsset>>>,

    mut views: Query<(
        &ExtractedView,
        &mut RenderPhase<ISPPhaseItem>,
        &VisibleEntities,
    )>,
) {
    let draw = volume_3d_draw_functions.read().id::<DrawISP>();

    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

    for (view, mut volume_phase, visible_ents) in &mut views {
        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();
        for &ent in &visible_ents.entities {
            let Ok((
                entity,
                mesh_uniform,
                mesh_handle
            )) = material_meshes.get(ent) else {continue};
            if let Some(mesh) = meshes.get(mesh_handle) {
                let key =
                    view_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                let pipeline = pipelines
                    .specialize(&pipeline_cache, &volume_pipeline, key, &mesh.layout)
                    .unwrap();
                volume_phase.add(ISPPhaseItem {
                    entity,
                    pipeline,
                    draw_function: draw,
                    distance: rangefinder.distance(&mesh_uniform.transform),
                });
            }
        }
    }
}

#[derive(Resource)]
pub struct ISPPipeline {
    // shader: Handle<Shader>,
    // depth_layout: BindGroupLayout,
    // volume_layout: BindGroupLayout,
    // mesh_pipeline: MeshPipeline,
    // depth_sampler: Sampler,
}

impl FromWorld for ISPPipeline {
    fn from_world(world: &mut World) -> Self {
		todo!()
        // // let shader = VOLUME_RENDER_HANDLE.typed();

        // let mesh_pipeline = world.resource::<MeshPipeline>();
        // let render_device = world.resource::<RenderDevice>();

        // let depth_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        //     label: None,
        //     entries: &[
        //         BindGroupLayoutEntry {
        //             binding: 0,
        //             visibility: ShaderStages::FRAGMENT,
        //             ty: BindingType::Texture {
        //                 sample_type: TextureSampleType::Depth,
        //                 view_dimension: TextureViewDimension::D2,
        //                 multisampled: false,
        //             },
        //             count: None,
        //         },
        //         BindGroupLayoutEntry {
        //             binding: 1,
        //             visibility: ShaderStages::FRAGMENT,
        //             ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
        //             count: None,
        //         },
        //         BindGroupLayoutEntry {
        //             binding: 2,
        //             visibility: ShaderStages::FRAGMENT,
        //             ty: BindingType::StorageTexture {
        //                 access: StorageTextureAccess::WriteOnly,
        //                 format: TextureFormat::R32Float,
        //                 view_dimension: TextureViewDimension::D2Array,
        //             },
        //             count: None,
        //         },
        //     ],
        // });
        // let depth_sampler = render_device.create_sampler(&Default::default());

        // let volume_layout = ISPRenderAsset::bind_group_layout(render_device);

        // ISPPipeline {
        //     shader,
        //     depth_layout,
        //     volume_layout,
        //     depth_sampler,
        //     mesh_pipeline: mesh_pipeline.clone(),
        // }
    }
}

impl SpecializedMeshPipeline for ISPPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
		todo!()
        // let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        // descriptor.label = Some("my pipeline".into());

        // descriptor.depth_stencil = None;

        // // From the example that I orignally built this ontop of:
        // // "meshes typically live in bind group 2. because we are using bindgroup 1
        // // we need to add MESH_BINDGROUP_1 shader def so that the bindings are correctly
        // // linked in the shader"
        // // No idea why we can't put them in bindgroup 2, tried it but it didn't work. It works now so
        // // decided not to mess with it.
        // descriptor
        //     .vertex
        //     .shader_defs
        //     .push("MESH_BINDGROUP_1".into());

        // descriptor.primitive.cull_mode = Some(Face::Front);

        // let fragment_state = descriptor.fragment.as_mut().unwrap();
        // fragment_state.shader = self.shader.clone();

        // fragment_state
        //     .targets
        //     .first_mut()
        //     .unwrap()
        //     .as_mut()
        //     .unwrap()
        //     .blend = Some(BlendState {
        //     color: BlendComponent {
        //         src_factor: BlendFactor::SrcAlpha,
        //         dst_factor: BlendFactor::DstAlpha,
        //         operation: BlendOperation::Max,
        //     },
        //     alpha: BlendComponent {
        //         src_factor: BlendFactor::SrcAlpha,
        //         dst_factor: BlendFactor::DstAlpha,
        //         operation: BlendOperation::Max,
        //     },
        // });

        // descriptor.layout.insert(2, self.depth_layout.clone());
        // descriptor.layout.insert(3, self.volume_layout.clone());
        // Ok(descriptor)
    }
}

type DrawISP = DrawMesh;
// type DrawVolume = (
//     SetItemPipeline,
//     SetMeshViewBindGroup<0>,
//     SetMeshBindGroup<1>,
//     SetStaticBindGroup<2>,
//     SetVolumeBindGroup<3>,
//     DrawMesh,
// );

struct SetStaticBindGroup<const I: usize>;
impl<const I: usize> RenderCommand<ISPPhaseItem> for SetStaticBindGroup<I> {
    type Param = ();

    type ViewWorldQuery = Read<DepthBindGroup>;

    type ItemWorldQuery = ();

    fn render<'w>(
        _item: &ISPPhaseItem,
        bind_group: ROQueryItem<'w, Self::ViewWorldQuery>,
        _entity: ROQueryItem<'w, Self::ItemWorldQuery>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &bind_group.value, &[]);
        RenderCommandResult::Success
    }
}

struct SetISPBindGroup<const I: usize>;
impl<const I: usize> RenderCommand<ISPPhaseItem> for SetISPBindGroup<I> {
    type Param = SRes<RenderAssets<ISPRenderAsset>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<Handle<ISPRenderAsset>>;

    #[inline]
    fn render<'w>(
        _item: &ISPPhaseItem,
        _view: (),
        handle: ROQueryItem<Self::ItemWorldQuery>,
        prepared: SystemParamItem<Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let prepared_isp = prepared.get(handle).unwrap();
        let illegal = unsafe { std::mem::transmute(&prepared_isp.bind_group) };
        pass.set_bind_group(I, illegal, &[]);
        RenderCommandResult::Success
    }
}

#[derive(Component)]
struct ISPRenderAssetTextures {
    // render_target: CachedTexture,
    depth_texture: CachedTexture,
    debug_texture: CachedTexture,
}

#[derive(Component)]
struct DepthBindGroup {
    value: BindGroup,
}

// fn prepare_render_textures(
//     mut commands: Commands,
//     mut texture_cache: ResMut<TextureCache>,
//     isp_pipeline: Res<ISPPipeline>,
//     render_device: Res<RenderDevice>,
//     views_3d: Query<(Entity, &ExtractedCamera), With<RenderPhase<ISPPhaseItem>>>,
// ) {
//     let mut textures = HashMap::default();

//     for (entity, camera) in &views_3d {
//         let Some(physical_target_size) = camera.physical_target_size else {
//             continue;
//         };
//         let size = Extent3d {
//             depth_or_array_layers: 1,
//             width: physical_target_size.x,
//             height: physical_target_size.y,
//         };

//         let (depth, debug) = textures
//             .entry(camera.target.clone())
//             .or_insert_with(|| {
//                 let depth = {
//                     let usage = TextureUsages::TEXTURE_BINDING
//                         | TextureUsages::COPY_DST
//                         | TextureUsages::COPY_SRC;

//                     let descriptor = TextureDescriptor {
//                         label: Some("volume_depth_buffer"),
//                         size,
//                         mip_level_count: 1,
//                         sample_count: 1,
//                         dimension: TextureDimension::D2,
//                         format: TextureFormat::Depth32Float,
//                         usage,
//                         view_formats: &[],
//                     };
//                     texture_cache.get(&render_device, descriptor)
//                 };

//                 let debug = {
//                     let size = Extent3d {
//                         depth_or_array_layers: 8,
//                         ..size
//                     };
//                     let usage = TextureUsages::STORAGE_BINDING
//                         | TextureUsages::COPY_DST
//                         | TextureUsages::COPY_SRC;

//                     let descriptor = TextureDescriptor {
//                         label: Some("volume_depth_buffer"),
//                         size,
//                         mip_level_count: 1,
//                         sample_count: 1,
//                         dimension: TextureDimension::D2,
//                         format: TextureFormat::R32Float,
//                         usage,
//                         view_formats: &[],
//                     };
//                     texture_cache.get(&render_device, descriptor)
//                 };
//                 (depth, debug)
//             })
//             .clone();

//         let depth_bindgroup_entries = [
//             BindGroupEntry {
//                 binding: 0,
//                 resource: BindingResource::TextureView(&depth.default_view),
//             },
//             BindGroupEntry {
//                 binding: 1,
//                 resource: BindingResource::Sampler(&volume_pipeline.depth_sampler),
//             },
//             BindGroupEntry {
//                 binding: 2,
//                 resource: BindingResource::TextureView(&debug.default_view),
//             },
//         ];

//         let depth_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
//             label: None,
//             layout: &volume_pipeline.depth_layout,
//             entries: &depth_bindgroup_entries,
//         });

//         commands
//             .entity(entity)
//             .insert(ISPRenderAssetTextures {
//                 depth_texture: depth,
//                 debug_texture: debug,
//             })
//             .insert(DepthBindGroup {
//                 value: depth_bind_group,
//             });
//     }
// }

