use std::marker::PhantomData;

use gpwgpu::{
    automatic_buffers::{AbstractBuffer, MemoryReq, SequentialOperation},
    parse_shaders_dyn,
    shaderpreprocessor::ShaderSpecs,
    utils::FullComputePass,
    wgpu::BufferUsages,
    ExpansionError,
};

use crate::setup::{InputType, Params};

parse_shaders_dyn!(SHADERS, "src/shaders");

#[derive(Hash, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Buffers {
    Bayered,
    Debayered,
}

impl Buffers {
    fn init<I: InputType>(self, params: &Params<I>) -> AbstractBuffer<Self> {
        let name = self;
        match self {
            Buffers::Bayered => AbstractBuffer {
                name,
                memory_req: MemoryReq::Strict,
                usage: BufferUsages::MAP_WRITE
                    | BufferUsages::STORAGE
                    | BufferUsages::COPY_DST
                    | BufferUsages::COPY_SRC,
                size: params.byte_size() as u64,
            },
            Buffers::Debayered => AbstractBuffer {
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

impl<I: InputType> SequentialOperation for Debayer<I> {
    type Params = Params<I>;

    type BufferEnum = Buffers;

    type Error = ExpansionError;

    type Args = ();

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
            Buffers::Bayered.init(params),
            Buffers::Debayered.init(params),
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
        let bayered = buffers.get::<Self>(Buffers::Bayered);
        let debayered = buffers.get::<Self>(Buffers::Debayered);

        let dispatch_size = [params.height as u32, params.width as u32, 1];

        let specs = ShaderSpecs::new((8, 32, 1))
            .direct_dispatcher(&dispatch_size)
            .extend_defs([
                ("TY", I::wgsl_type().into()),
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
        _args: &Self::Args,
    ) {
        self.pass.execute(encoder, &[]);
    }
}
