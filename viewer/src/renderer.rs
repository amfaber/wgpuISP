use std::sync::Arc;

use bevy::{
    core_pipeline::{core_2d, core_3d},
    ecs::{
        query::{QueryItem, ROQueryItem},
        system::{
            lifetimeless::{Read, SRes, SResMut},
            SystemParamItem,
        },
    },
    pbr::{DrawMesh, MeshPipelineKey},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{
        extract_component::ExtractComponentPlugin,
        mesh::MeshVertexBufferLayout,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner,
        },
        render_phase::{
            AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions,
            PhaseItem, RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::{DefaultImageSampler, GpuImage},
        view::{ExtractedView, VisibleEntities},
        Extract, Render, RenderApp, RenderSet,
    },
    sprite::{
        DrawMesh2d, Mesh2dPipeline, Mesh2dPipelineKey, SetMesh2dBindGroup, SetMesh2dViewBindGroup,
    },
    utils::FloatOrd,
};
use gpwgpu::{
    bytemuck,
    utils::{DebugBundle, DebugEncoder, InspectBuffer},
    wgpu,
};
use wgpu_isp::{
    operations::Buffers,
    setup::{ISPParams, Params as SetupParams, State as ISPState},
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
            ExtractComponentPlugin::<Handle<BevyISPState>>::default(),
            RenderAssetPlugin::<BevyISPState>::default(),
            ExtractComponentPlugin::<Handle<BevyISPParams>>::default(),
            RenderAssetPlugin::<BevyISPParams>::default(),
            ExtractComponentPlugin::<Handle<ISPImage>>::default(),
            RenderAssetPlugin::<ISPImage>::default(),
        ))
        .insert_resource(Msaa::Off)
        .add_asset::<BevyISPState>()
        .add_asset::<BevyISPParams>()
        .add_asset::<ISPImage>();

        app.sub_app_mut(RenderApp)
            .init_resource::<DrawFunctions<ISPPhaseItem>>()
            .add_render_command::<ISPPhaseItem, DrawISP>()
            // .init_resource::<SpecializedMeshPipelines<ISPPipeline>>()
            .init_resource::<RenderAssets<BevyISPState>>()
            .init_resource::<RenderAssets<BevyISPParams>>()
            .init_resource::<RenderAssets<ISPImage>>()
            .add_systems(ExtractSchedule, extract_camera_isp_phase)
            .add_systems(Render, (queue.in_set(RenderSet::Queue),))
            .add_render_graph_node::<ViewNodeRunner<ISPNode>>(core_2d::graph::NAME, ISPNode::NAME)
            .add_render_graph_edges(
                core_2d::graph::NAME,
                &[
                    core_2d::graph::node::MAIN_PASS,
                    ISPNode::NAME,
                    core_2d::graph::node::END_MAIN_PASS_POST_PROCESSING,
                ],
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<ISPPipeline>();
    }
}

pub struct SendState(ISPState<'static>);

// Kinda illegal, but I believe it works in this context
unsafe impl Send for SendState {}
unsafe impl Sync for SendState {}

#[derive(TypePath, TypeUuid, Clone)]
#[uuid = "6ea266a6-6cf3-53a4-9986-1d7bf5c12396"]
pub struct BevyISPState(pub SetupParams);

impl RenderAsset for BevyISPState {
    type ExtractedAsset = Self;

    type PreparedAsset = SendState;

    type Param = (SRes<RenderDevice>, SRes<RenderQueue>);

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (device, queue): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        unsafe {
            let device = std::mem::transmute::<&wgpu::Device, &wgpu::Device>(device.wgpu_device());
            let queue = std::mem::transmute::<&wgpu::Queue, &wgpu::Queue>(queue);
            let state = ISPState::new(device, queue, extracted_asset.0).unwrap();
            Ok(SendState(state))
        }
    }
}

#[derive(TypePath, TypeUuid, Clone)]
#[uuid = "6ea266a6-6cf3-53a4-9986-1d5bf5c12396"]
pub struct ISPImage {
    pub data: Arc<Vec<f32>>,
    pub texture_desc: TextureDescriptor<'static>,
    pub state: Handle<BevyISPState>,
    pub height: i32,
    pub width: i32,
}

impl RenderAsset for ISPImage {
    type ExtractedAsset = Self;

    type PreparedAsset = GpuImage;

    type Param = (
        SResMut<RenderAssets<BevyISPState>>,
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<DefaultImageSampler>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (state_assets, device, queue, default_sampler): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        if let Some(state) = state_assets.get(&extracted_asset.state) {
            let first_buf = state.0.sequential.buffers.get_from_any(Buffers::Raw);

            queue.write_buffer(first_buf, 0, bytemuck::cast_slice(&extracted_asset.data));

            queue.submit(None);

            let texture = device.create_texture(&extracted_asset.texture_desc);

            let texture_view = texture.create_view(&default());

            let gpu_image = GpuImage {
                texture,
                texture_view,
                texture_format: extracted_asset.texture_desc.format,
                sampler: (***default_sampler).clone(),
                size: Vec2::new(extracted_asset.height as f32, extracted_asset.width as f32),
                mip_level_count: 1,
            };
            Ok(gpu_image)
        } else {
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset))
        }
    }
}

#[derive(TypePath, TypeUuid, Clone, Debug)]
#[uuid = "61a266a6-6cf3-53a4-9986-1d7bf5c12396"]
pub struct BevyISPParams(pub ISPParams);

impl RenderAsset for BevyISPParams {
    type ExtractedAsset = Self;

    type PreparedAsset = Self;

    type Param = ();

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        Ok(extracted_asset)
    }
}

#[derive(Default)]
struct ISPNode;

impl ISPNode {
    const NAME: &str = "isp";
}

impl ViewNode for ISPNode {
    type ViewQuery = Read<RenderPhase<ISPPhaseItem>>;
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        render_phase: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        for item in &render_phase.items {
            let ent = item.entity;
            let Some(state) = world.get::<Handle<BevyISPState>>(ent) else {
                continue
            };
            let Some(state) = world.resource::<RenderAssets<BevyISPState>>().get(state) else {
                continue
            };

            let Some(texture) = world.get::<Handle<ISPImage>>(ent) else {
                continue
            };
            let Some(texture) = world.resource::<RenderAssets<ISPImage>>().get(texture) else {
                continue
            };

            let Some(params) = world.get::<Handle<BevyISPParams>>(ent) else {
                continue
            };
            let Some(params) = world.resource::<RenderAssets<BevyISPParams>>().get(params) else {
                continue
            };

            let mut encoder = DebugEncoder::new(&state.0.device);
            state.0.sequential.execute(&mut encoder, &params.0);


            encoder.submit(&state.0.queue);
        }

        let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("ISPnode"),
            color_attachments: &[],
            depth_stencil_attachment: None,
        });

        render_phase.render(&mut pass, world, graph.view_entity());
        Ok(())
    }
}

pub struct ISPPhaseItem {
    // pub distance: f32,
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
        FloatOrd(0.)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
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
    cameras_2d: Extract<Query<(Entity, &Camera), With<Camera2d>>>,
) {
    for (entity, camera) in &cameras_2d {
        if camera.is_active {
            commands
                .get_or_spawn(entity)
                .insert((RenderPhase::<ISPPhaseItem>::default(),));
        }
    }
}

fn queue(
    isp_draw_functions: Res<DrawFunctions<ISPPhaseItem>>,
    isp_pipeline: Res<ISPPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<ISPPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<Mesh>>,
    // material_meshes: Query<(Entity, &MeshUniform, &Handle<Mesh>), With<(Handle<BevyISPParams>,)>>,
    query: Query<(
        Entity,
        &Handle<ISPImage>,
        &Handle<BevyISPParams>,
        &Handle<BevyISPState>,
        &Handle<Mesh>,
    )>,

    mut views: Query<(
        &ExtractedView,
        &mut RenderPhase<ISPPhaseItem>,
        // &VisibleEntities,
    )>,
) {
    let draw = isp_draw_functions.read().id::<DrawISP>();

    let key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples());
    for (view, mut isp_phase) in &mut views {
        let key = Mesh2dPipelineKey::from_hdr(view.hdr) | key;
        for (entity, _, _, _, mesh) in &query {
            // let Ok((entity, _, _, _)) = query.get(entity) else {continue};
            let Some(mesh) = meshes.get(mesh) else { continue };
            let key = key | Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology);
            let Ok(pipeline) = pipelines.specialize(&pipeline_cache, &isp_pipeline, key, &mesh.layout) else { continue };
            isp_phase.add(ISPPhaseItem {
                pipeline,
                entity,
                draw_function: draw,
            });
        }
    }
}

#[derive(Resource)]
pub struct ISPPipeline {
    mesh_pipeline: Mesh2dPipeline,
}

impl FromWorld for ISPPipeline {
    fn from_world(world: &mut World) -> Self {
        Self {
            mesh_pipeline: Mesh2dPipeline::from_world(world),
        }
    }
}

impl SpecializedMeshPipeline for ISPPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let descriptor = self.mesh_pipeline.specialize(key, layout)?;
        Ok(descriptor)
    }
}

// type DrawISP = ();
type DrawISP = (
    // Set the pipeline
    SetItemPipeline,
    // Set the view uniform as bind group 0
    SetMesh2dViewBindGroup<0>,
    // Set the mesh uniform as bind group 1
    SetMesh2dBindGroup<1>,
    // Draw the mesh
    DrawMesh2d,
);

// struct SetISPBindGroup<const I: usize>;
// impl<const I: usize> RenderCommand<ISPPhaseItem> for SetISPBindGroup<I> {
//     type Param = ();
//     // type Param = SRes<RenderAssets<()>>;
//     type ViewWorldQuery = ();
//     type ItemWorldQuery = Read<Handle<()>>;

//     #[inline]
//     fn render<'w>(
//         _item: &ISPPhaseItem,
//         _view: (),
//         handle: ROQueryItem<Self::ItemWorldQuery>,
//         prepared: SystemParamItem<Self::Param>,
//         pass: &mut TrackedRenderPass<'w>,
//     ) -> RenderCommandResult {
//         // let prepared_isp = prepared.get(handle).unwrap();
//         // let illegal = unsafe { std::mem::transmute(&prepared_isp.bind_group) };
//         // pass.set_bind_group(I, illegal, &[]);
//         // RenderCommandResult::Success
//         todo!()
//     }
// }
