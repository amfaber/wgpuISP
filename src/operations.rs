use std::marker::PhantomData;

use gpwgpu::{
    automatic_buffers::{AbstractBuffer, MemoryReq, SequentialOperation},
    parse_shaders_dyn,
    shaderpreprocessor::ShaderSpecs,
    utils::FullComputePass,
    wgpu::BufferUsages,
    ExpansionError, bytemuck,
};

use crate::setup::{InputType, Params};

parse_shaders_dyn!(SHADERS, "src/shaders");

#[derive(Hash, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Buffers {
    Raw,
    BlackLevel,
    RGB,

}

impl Buffers {
    fn init<I: InputType>(self, params: &Params<I>) -> AbstractBuffer<Self> {
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
                usage: BufferUsages::STORAGE
                    | BufferUsages::COPY_DST
                    | BufferUsages::COPY_SRC,
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
pub struct Debayer<I: InputType> {
    pass: FullComputePass,
    phan: PhantomData<I>,
}

#[derive(Debug)]
pub struct DebayerParams{
    pub enabled: bool,
}

impl<I: InputType> SequentialOperation for Debayer<I> {
    type Params = Params<I>;

    type BufferEnum = Buffers;

    type Error = ExpansionError;

    type Args = Params<I>;

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
        vec![
            Buffers::BlackLevel.init(params),
            Buffers::RGB.init(params),
        ]
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

        let shader = SHADERS.process_by_name("debayer", specs)?;

        let pipeline = shader.build(device);

        let bindgroup = [(0, bayered), (1, debayered)];

        let pass = FullComputePass::new(device, pipeline, &bindgroup);

        Ok(Self {
            pass,
            phan: PhantomData,
        })
    }

    fn execute(
        &mut self,
        encoder: &mut gpwgpu::utils::Encoder,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<Self::BufferEnum>,
        args: &Self::Args,
    ) {
        if !args.isp.debayer.enabled{
            return
        }
        self.pass.execute(encoder, &[]);
    }
}


#[derive(Debug)]
pub struct BlackLevel<I: InputType>{
    pass: FullComputePass,
    phan: PhantomData<I>,
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct BlackLevelPush{
	pub r_offset: f32,
	pub gr_offset: f32,
	pub gb_offset: f32,
	pub b_offset: f32,
	pub alpha: f32,
	pub beta: f32,
}

#[derive(Debug)]
pub struct BlackLevelParams{
    pub enabled: bool,
    pub push: BlackLevelPush,
}

impl<I: InputType> SequentialOperation for BlackLevel<I>{
    type Params = Params<I>;

    type BufferEnum = Buffers;

    type Error = ExpansionError;

    type Args = Params<I>;

    fn enabled(_params: &Self::Params) -> bool
    where
        Self: Sized {
        true
    }

    fn buffers(params: &Self::Params) -> Vec<AbstractBuffer<Self::BufferEnum>>
    where
        Self: Sized {
        vec![
            Buffers::Raw.init(params),
            Buffers::BlackLevel.init(params),
        ]
    }

    fn create(
        device: &gpwgpu::wgpu::Device,
        params: &Self::Params,
        buffers: &gpwgpu::automatic_buffers::BufferSolution<Self::BufferEnum>,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized {
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

        let shader = SHADERS.process_by_name("black_level", specs)?;

        let pipeline = shader.build(device);

        let bindgroup = [(0, raw), (1, black_level)];

        let pass = FullComputePass::new(device, pipeline, &bindgroup);

        Ok(Self {
            pass,
            phan: PhantomData,
        })
        
    }

    fn execute(
        &mut self,
        encoder: &mut gpwgpu::utils::Encoder,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<Self::BufferEnum>,
        args: &Self::Args,
    ) {
        if !args.isp.black_level.enabled{
            return
        }
        self.pass.execute(encoder, bytemuck::bytes_of(&args.isp.black_level.push))
    }
}

