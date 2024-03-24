use gpwgpu::{
    automatic_buffers::{AllOperations, Operation},
    shaderpreprocessor::ShaderProcessor,
    utils::{DebugBundle, FullComputePass, InspectBuffer},
    wgpu::{Device, Extent3d, Queue, Texture, TextureDescriptor, TextureDimension, TextureUsages},
};

use crate::operations::{
    create_to_texture, AutoWhiteBalance, BlackLevel, Buffers, Debayer, ISPParams, PreserveRaw, RGBSpaceOperations, StateError, PT
};

#[derive(Debug, Clone)]
pub struct Params {
    pub width: i32,
    pub height: i32,

    pub shader_processor: ShaderProcessor<'static>,
}

impl Params {
    pub fn byte_size(&self) -> i32 {
        self.width * self.height * std::mem::size_of::<f32>() as i32
    }
}

pub struct State<'a> {
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub params: Params,
    pub to_texture: FullComputePass,
    pub texture: Texture,
    pub sequential: AllOperations<PT>,
}

impl<'a> State<'a> {
    pub fn new(device: &'a Device, queue: &'a Queue, params: Params) -> Result<Self, StateError> {
        let operations = vec![
            Operation::new::<BlackLevel>(),
            Operation::new::<AutoWhiteBalance>(),
            Operation::new::<Debayer>(),
            Operation::new::<RGBSpaceOperations>(),
            Operation::new::<PreserveRaw>(),
        ];

        let mut sequential = AllOperations::new(&params, operations)?;
        sequential.finalize(device, &params)?;

        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: params.width as _,
                height: params.height as _,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: gpwgpu::wgpu::TextureFormat::Rgba32Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let to_texture = create_to_texture(
            device,
            &params,
            sequential.buffers.get_from_any(Buffers::RGB),
            &texture,
        )?;

        Ok(Self {
            device,
            queue,
            params,
            sequential,
            to_texture,
            texture,
        })
    }

    pub fn write_to_input(&self, data: &[f32]) {
        let buf = self.sequential.buffers.get_from_any(Buffers::Raw);
        self.queue.write_buffer(buf, 0, bytemuck::cast_slice(data));
    }

    pub fn reload(&self, params: Params) -> Result<Self, StateError> {
        Self::new(&self.device, &self.queue, params)
    }
}

#[allow(unused)]
pub fn make_debug_bundle<'a>(state: &'a State<'a>) -> DebugBundle<'a> {
    DebugBundle {
        device: &state.device,
        queue: &state.queue,
        inspects: vec![
            InspectBuffer::new(
                state.sequential.buffers.get_from_any(Buffers::Raw),
                None,
                "input",
            ),
            InspectBuffer::new(
                state.sequential.buffers.get_from_any(Buffers::BlackLevel),
                None,
                "black_level",
            ),
            InspectBuffer::new(
                state.sequential.buffers.get_from_any(Buffers::TempMean),
                None,
                "temp_mean",
            ),
            InspectBuffer::new(
                state.sequential.buffers.get_from_any(Buffers::Mean),
                None,
                "mean",
            ),
            InspectBuffer::new(
                state.sequential.buffers.get_from_any(Buffers::RGB),
                None,
                "output",
            ),
        ],
        save_path: "tests/dumps".into(),
        create_py: true,
    }
}

// fn compile_check<T: Send + Sync>(){
//     fn test<T: Send>(){

//     }

//     test::<State>();
// }
