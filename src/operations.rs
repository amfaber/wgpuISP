use std::marker::PhantomData;

use gpwgpu::{automatic_buffers::{SequentialOperation, AbstractBuffer, MemoryReq}, wgpu::BufferUsages, parse_shaders_dyn, shaderpreprocessor::ShaderSpecs, ExpansionError};

use crate::setup::{Params, InputType};

parse_shaders_dyn!(SHADERS, "src/shaders");


#[derive(Hash, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Buffers{
    Bayered,
    Debayered,
}

impl Buffers{
    fn init<I: InputType>(self, params: &Params<I>) -> AbstractBuffer<Self>{
        let name = self;
        match self{
            Buffers::Bayered => AbstractBuffer{
                name,
                memory_req: MemoryReq::Strict,
                usage: BufferUsages::MAP_WRITE | BufferUsages::STORAGE | BufferUsages::COPY_DST,
                size: params.byte_size() as u64,
            },
            Buffers::Debayered => AbstractBuffer{
                name,
                memory_req: MemoryReq::Strict,
                usage: BufferUsages::MAP_READ | BufferUsages::STORAGE | BufferUsages::COPY_DST,
                size: params.byte_size() as u64,
            },
        }
    }
}



#[derive(Debug)]
pub struct Debayer<I: InputType>{

    phan: PhantomData<I>
}



impl<I: InputType> SequentialOperation for Debayer<I>{
    type Params = Params<I>;

    type BufferEnum = Buffers;

    type Error = ExpansionError;

    type Args = ();

    fn enabled(_params: &Self::Params) -> bool
    where
        Self: Sized {
        true
    }

    fn buffers(params: &Self::Params) -> Vec<gpwgpu::automatic_buffers::AbstractBuffer<Self::BufferEnum>>
    where
        Self: Sized {
        vec![
            Buffers::Bayered.init(params),
            Buffers::Debayered.init(params),
        ]
    }

    fn create(
        _device: &gpwgpu::wgpu::Device,
        _params: &Self::Params,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<Self::BufferEnum>,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized {

        let specs = ShaderSpecs::new((16, 16, 1))
            .extend_defs([
                ("TY", I::wgsl_type().into()),
                // ("TY", I::wgsl_type().into()),
            ]);
        let shader = SHADERS.process_by_name("debayer", specs)?;

        dbg!(&shader);

        Ok(Self{phan: PhantomData})
    }

    fn execute(
        &mut self,
        _encoder: &mut gpwgpu::utils::Encoder,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<Self::BufferEnum>,
        _args: &Self::Args,
    ) {
        todo!()
    }
}
