use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
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
        extract_resource,
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
            CachedTexture, FallbackImage, ImageSampler, TextureCache, TextureFormatPixelInfo,
        },
        view::{ExtractedView, ViewDepthTexture, ViewTarget, VisibleEntities},
        Extract, Render, RenderApp, RenderSet,
    },
    utils::{FloatOrd, HashMap},
};
use gpwgpu::{automatic_buffers::AllOperations, bytemuck, wgpu, ExpansionError, utils::DebugEncoder};
use wgpu_isp::{
    operations::Buffers,
    setup::{ISPParams, InputType, Params as SetupParams, State as ISPState},
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

        // app.add_plugins((
        //     ExtractComponentPlugin::<Handle<ISPRenderAsset>>::default(),
        //     RenderAssetPlugin::<ISPRenderAsset>::default(),
        // ))
        // .insert_resource(Msaa::Off)
        // .add_asset::<ISPRenderAsset>();

        // app.sub_app_mut(RenderApp)
        //     .init_resource::<DrawFunctions<ISPPhaseItem>>()
        //     .add_render_command::<ISPPhaseItem, DrawISP>()
        //     .init_resource::<SpecializedMeshPipelines<ISPPipeline>>()
        //     .init_resource::<RenderAssets<ISPRenderAsset>>()
        //     .add_systems(ExtractSchedule, extract_camera_isp_phase)
        //     .add_systems(
        //         Render,
        //         (
        //             queue.in_set(RenderSet::Queue),
        //             // prepare_render_textures
        //                 // .in_set(RenderSet::Prepare)
        //                 // .after(render::view::prepare_windows),
        //         ),
        //     )
        //     .add_render_graph_node::<ViewNodeRunner<ISPNode>>(
        //         core_3d::graph::NAME,
        //         ISPNode::NAME,
        //     )
        //     .add_render_graph_edges(
        //         core_3d::graph::NAME,
        //         &[
        //             core_3d::graph::node::MAIN_TRANSPARENT_PASS,
        //             ISPNode::NAME,
        //             core_3d::graph::node::END_MAIN_PASS,
        //         ],
        //     );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<ISPPipeline>();
    }
}

// #[derive(Debug, Clone, ShaderType, Default, TypePath, TypeUuid)]
// #[uuid = "6ea266a6-6cf3-53a4-9287-1dabf5c17d6f"]
// pub struct RaycastInput {
//     pub bounding_box: [Vec3; 2],
//     pub cmap_bounds: Vec2,
// }

// enum UpdatedSignal {
//     MainWorld(AtomicBool),
//     RenderWorld(bool),
// }

// pub struct Updated<T: Clone> {
//     pub data: T,
//     updated: UpdatedSignal,
// }

// impl<T: Clone> Updated<T> {
//     fn new(data: T, updated: bool) -> Self {
//         Self {
//             data,
//             updated: UpdatedSignal::MainWorld(AtomicBool::new(updated)),
//         }
//     }

//     fn take_clone(&self) -> Self {
//         let UpdatedSignal::MainWorld(atomic) = &self.updated else {
//             unreachable!("take_clone should only be called on updates from the
//                 main world");
//         };
//         let updated = atomic.swap(false, Ordering::SeqCst);
//         Self {
//             data: self.data.clone(),
//             updated: UpdatedSignal::RenderWorld(updated),
//         }
//     }
// }

pub struct SendState(ISPState<'static, u16>);

// Kinda illegal, but I believe it works in this context
unsafe impl Send for SendState {}
unsafe impl Sync for SendState {}

#[derive(TypePath, TypeUuid, Clone)]
#[uuid = "6ea266a6-6cf3-53a4-9986-1d7bf5c12396"]
pub struct BevyISPState(SetupParams<u16>);

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
    pub data: Arc<Vec<u16>>,
    pub states: Vec<Handle<BevyISPState>>,
}

impl RenderAsset for ISPImage {
    type ExtractedAsset = Self;

    type PreparedAsset = ();

    type Param = (
        SResMut<RenderAssets<BevyISPState>>,
        SRes<RenderDevice>,
        SRes<RenderQueue>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        mut extracted_asset: Self::ExtractedAsset,
        (state_assets, device, queue): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let mut ready = Vec::new();
        let mut not_ready = Vec::new();
        for state in extracted_asset.states.into_iter() {
            if state_assets.contains_key(&state) {
                ready.push(state)
            } else {
                not_ready.push(state)
            }
        }
        let mut state_iter = ready.iter();
        if let Some(state) = state_iter.next() {
            let asset = state_assets.get(state).unwrap();
            let first_buf = asset.0.sequential.buffers.get_from_any(Buffers::Raw);

            queue.write_buffer(first_buf, 0, bytemuck::cast_slice(&extracted_asset.data));

            let mut encoder = device.create_command_encoder(&Default::default());
            for next_state in state_iter {
                let asset = state_assets.get(next_state).unwrap();
                let buf = asset.0.sequential.buffers.get_from_any(Buffers::Raw);
                encoder.copy_buffer_to_buffer(first_buf, 0, buf, 0, first_buf.size());
            }
            queue.submit(Some(encoder.finish()));
        }
        if !not_ready.is_empty() {
            extracted_asset.states = not_ready;
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset))
        } else {
            Ok(())
        }
    }
}

#[derive(TypePath, TypeUuid, Clone)]
#[uuid = "61a266a6-6cf3-53a4-9986-1d7bf5c12396"]
pub struct BevyISPParams(ISPParams);

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

fn queue(// isp_draw_functions: Res<DrawFunctions<ISPPhaseItem>>,
    // isp_pipeline: Res<ISPPipeline>,
    // msaa: Res<Msaa>,
    // mut pipelines: ResMut<SpecializedMeshPipelines<ISPPipeline>>,
    // pipeline_cache: Res<PipelineCache>,
    // meshes: Res<RenderAssets<Mesh>>,
    // material_meshes: Query<(Entity, &MeshUniform, &Handle<Mesh>), With<Handle<ISPRenderAsset>>>,

    // mut views: Query<(
    //     &ExtractedView,
    //     &mut RenderPhase<ISPPhaseItem>,
    //     &VisibleEntities,
    // )>,
) {
    // let draw = isp_draw_functions.read().id::<DrawISP>();

    // let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

    // for (view, mut isp_phase, visible_ents) in &mut views {
    //     let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
    //     let rangefinder = view.rangefinder3d();
    //     for &ent in &visible_ents.entities {
    //         let Ok((
    //             entity,
    //             mesh_uniform,
    //             mesh_handle
    //         )) = material_meshes.get(ent) else {continue};
    //         if let Some(mesh) = meshes.get(mesh_handle) {
    //             let key =
    //                 view_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
    //             let pipeline = pipelines
    //                 .specialize(&pipeline_cache, &isp_pipeline, key, &mesh.layout)
    //                 .unwrap();
    //             isp_phase.add(ISPPhaseItem {
    //                 entity,
    //                 pipeline,
    //                 draw_function: draw,
    //                 distance: rangefinder.distance(&mesh_uniform.transform),
    //             });
    //         }
    //     }
    // }
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

type DrawISP = (ExecutePipeline);
// type DrawVolume = (
//     SetItemPipeline,
//     SetMeshViewBindGroup<0>,
//     SetMeshBindGroup<1>,
//     SetStaticBindGroup<2>,
//     SetVolumeBindGroup<3>,
//     DrawMesh,
// );

struct ExecutePipeline;
impl RenderCommand<ISPPhaseItem> for ExecutePipeline {
    type Param = (
        SRes<RenderAssets<BevyISPState>>,
        // SRes<RenderAssets<ISPImage>>,
        SRes<RenderAssets<BevyISPParams>>,
    );

    type ViewWorldQuery = ();

    type ItemWorldQuery = (
        Read<Handle<BevyISPState>>,
        // Read<Handle<ISPImage>>,
        Read<Handle<BevyISPParams>>,
    );

    fn render<'w>(
        _item: &ISPPhaseItem,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        (state, params): ROQueryItem<'w, Self::ItemWorldQuery>,
        (states, asset_params): SystemParamItem<'w, '_, Self::Param>,
        _pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(state) = states.get(state) else { return RenderCommandResult::Failure };
        let Some(params) = asset_params.get(params) else { return RenderCommandResult::Failure };

        let mut encoder = DebugEncoder::new(&state.0.device);
        state.0.sequential.execute(&mut encoder, &params.0);
        encoder.submit(&state.0.queue);
        
        RenderCommandResult::Success
    }
}

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

