use bytemuck::bytes_of;
use gpwgpu::{
    automatic_buffers::{AbstractBuffer, MemoryReq, SequentialOperation},
    bytemuck, parse_shaders,
    shaderpreprocessor::ShaderSpecs,
    utils::FullComputePass,
    wgpu::{BindGroupEntry, BufferUsages, Device, Texture, Buffer},
    ExpansionError,
};
use macros::UiMarker;

use crate::setup::{ISPParams, Params};

parse_shaders!(pub SHADERS, "src/shaders");

#[derive(Hash, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Buffers {
    Raw,
    BlackLevel,
    RGB,
}

impl Buffers {
    fn init(self, params: &Params) -> AbstractBuffer<Self> {
        let name = self;
        match self {
            Buffers::Raw => AbstractBuffer {
                name,
                memory_req: MemoryReq::Strict,
                usage: BufferUsages::MAP_WRITE
                    | BufferUsages::STORAGE
                    | BufferUsages::COPY_DST
                    | BufferUsages::COPY_SRC,
                size: params.byte_size() as u64,
            },
            Buffers::BlackLevel => AbstractBuffer {
                name,
                memory_req: MemoryReq::Strict,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
                size: params.byte_size() as u64,
            },
            Buffers::RGB => AbstractBuffer {
                name,
                memory_req: MemoryReq::Strict,
                usage: BufferUsages::MAP_READ | BufferUsages::STORAGE | BufferUsages::COPY_SRC,
                size: (params.byte_size() * 4) as u64,
            },
        }
    }
}

#[derive(Debug)]
pub struct Debayer {
    pass: FullComputePass,
}

#[derive(Debug, Clone)]
pub struct DebayerParams {
    pub enabled: bool,
}

impl SequentialOperation for Debayer {
    type Params = Params;

    type BufferEnum = Buffers;

    type Error = ExpansionError;

    type Args = ISPParams;

    fn enabled(_params: &Self::Params) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn buffers(
        params: &Self::Params,
    ) -> Vec<gpwgpu::automatic_buffers::AbstractBuffer<Self::BufferEnum>>
    where
        Self: Sized,
    {
        vec![Buffers::BlackLevel.init(params), Buffers::RGB.init(params)]
    }

    fn create(
        device: &gpwgpu::wgpu::Device,
        params: &Self::Params,
        buffers: &gpwgpu::automatic_buffers::BufferSolution<Self::BufferEnum>,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let bayered = buffers.get::<Self>(Buffers::BlackLevel);
        let debayered = buffers.get::<Self>(Buffers::RGB);

        let dispatch_size = [params.height as u32, params.width as u32, 1];

        let specs = ShaderSpecs::new((8, 32, 1))
            .direct_dispatcher(&dispatch_size)
            .extend_defs([
                ("HEIGHT", params.height.into()),
                ("WIDTH", params.width.into()),
                ("PADDING", 2.into()),
            ]);

        let shader = params.shader_processor.process_by_name("debayer", specs)?;

        let pipeline = shader.build(device);

        let bindgroup = [(0, bayered), (1, debayered)];

        let pass = FullComputePass::new(device, pipeline, &bindgroup);

        Ok(Self { pass })
    }

    fn execute(
        &mut self,
        encoder: &mut gpwgpu::utils::Encoder,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<Self::BufferEnum>,
        args: &Self::Args,
    ) {
        let push = if args.debayer.enabled{
            1i32
        } else {
            0
        };
        self.pass.execute(encoder, bytes_of(&push));
    }
}

#[derive(Debug)]
pub struct BlackLevel {
    pass: FullComputePass,
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, Default, UiMarker)]
#[repr(C)]
pub struct BlackLevelPush {
    #[ui(min = 1, max = 50)]
    pub r_offset: f32,
    pub gr_offset: f32,
    pub gb_offset: f32,
    pub b_offset: f32,
    pub alpha: f32,
    pub beta: f32,
}

#[derive(Debug, Clone)]
pub struct BlackLevelParams {
    pub enabled: bool,
    pub push: BlackLevelPush,
}

impl SequentialOperation for BlackLevel {
    type Params = Params;

    type BufferEnum = Buffers;

    type Error = ExpansionError;

    type Args = ISPParams;

    fn enabled(_params: &Self::Params) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn buffers(params: &Self::Params) -> Vec<AbstractBuffer<Self::BufferEnum>>
    where
        Self: Sized,
    {
        vec![Buffers::Raw.init(params), Buffers::BlackLevel.init(params)]
    }

    fn create(
        device: &gpwgpu::wgpu::Device,
        params: &Self::Params,
        buffers: &gpwgpu::automatic_buffers::BufferSolution<Self::BufferEnum>,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let raw = buffers.get::<Self>(Buffers::Raw);
        let black_level = buffers.get::<Self>(Buffers::BlackLevel);

        let dispatch_size = [params.height as u32, params.width as u32, 1];

        let specs = ShaderSpecs::new((8, 32, 1))
            .direct_dispatcher(&dispatch_size)
            .extend_defs([
                ("HEIGHT", params.height.into()),
                ("WIDTH", params.width.into()),
                ("PADDING", 1.into()),
            ]);

        let shader = params.shader_processor.process_by_name("black_level", specs)?;

        let pipeline = shader.build(device);

        let bindgroup = [(0, raw), (1, black_level)];

        let pass = FullComputePass::new(device, pipeline, &bindgroup);

        Ok(Self { pass })
    }

    fn execute(
        &mut self,
        encoder: &mut gpwgpu::utils::Encoder,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<Self::BufferEnum>,
        args: &Self::Args,
    ) {
        if !args.black_level.enabled {
            return;
        }
        self.pass
            .execute(encoder, bytemuck::bytes_of(&args.black_level.push))
    }
}

pub fn create_to_texture(
    device: &Device,
    params: &Params,
    final_buffer: &Buffer,
    texture: &Texture,
) -> Result<FullComputePass, ExpansionError> {
    let dispatch_size = [params.height as u32, params.width as u32, 1];
    
    let shader = params.shader_processor.process_by_name(
        "to_texture",
        ShaderSpecs::new((16, 16, 1))
            .extend_defs([
                ("HEIGHT", (params.height).into()),
                ("WIDTH", (params.width).into()),
                // ("HEIGHT", (params.height).into()),
                // ("WIDTH", (params.width).into()),
            ]).direct_dispatcher(&dispatch_size),
    )?;

    let pipeline = shader.build(device);

    let view = &texture.create_view(&Default::default());

    
    
    let bindgroup = [
        BindGroupEntry {
            binding: 0,
            resource: final_buffer.as_entire_binding(),
        },
        BindGroupEntry {
            binding: 1,
            resource: gpwgpu::wgpu::BindingResource::TextureView(
                &view,
            ),
        },
    ];

    let pass = FullComputePass::new(device, pipeline, &bindgroup);

    Ok(pass)
}
