use std::marker::PhantomData;

use gpwgpu::{automatic_buffers::{AllOperations, register}, ExpansionError, wgpu::{Device, Queue}};

use crate::operations::{Buffers, Debayer, BlackLevelParams, DebayerParams, BlackLevel};



pub trait InputType: Sized + std::fmt::Debug + 'static{
    fn wgsl_type() -> &'static str{
        std::any::type_name::<Self>()
    }
}

impl InputType for u16{
}

#[derive(Debug)]
pub struct ISPParams{
    pub debayer: DebayerParams,
    pub black_level: BlackLevelParams,
}

#[derive(Debug)]
pub struct Params<I: InputType>{
    pub width: i32,
    pub height: i32,

    pub isp: ISPParams,

    pub phan: PhantomData<I>
}

impl<I: InputType> Params<I>{
    
    pub fn byte_size(&self) -> i32{
        self.width * self.height * std::mem::size_of::<f32>() as i32
    }

}

type StateError = ExpansionError;

pub struct State<'a, I: InputType>{
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub params: Params<I>,
    pub sequential: AllOperations<Params<I>, Buffers, StateError, Params<I>>,
}

impl<'a, I: InputType> State<'a, I>{
    pub fn new(device: &'a Device, queue: &'a Queue, params: Params<I>) -> Result<Self, StateError> {


        let operations = vec![
            register::<BlackLevel<I>>(),
            register::<Debayer<I>>(),
        ];
        
        let mut sequential = AllOperations::new(&params, operations)?;
        sequential.finalize(device, &params)?;
        
        Ok(Self{
            device,
            queue,
            params,
            sequential,
        })
    }
}
