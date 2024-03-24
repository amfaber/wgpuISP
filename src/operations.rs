use std::mem::size_of;

use bytemuck::bytes_of;
use gpwgpu::{
    automatic_buffers::{AbstractBuffer, MemoryReq, PipelineArgs, PipelineError, PipelineParams, PipelineTypes, SequentialOperation},
    bytemuck,
    operations::reductions::{InputType, MeanReduce},
    shaderpreprocessor::{ShaderError, ShaderSpecs},
    utils::FullComputePass,
    wgpu::{BindGroupEntry, Buffer, BufferUsages, Device, Texture},
};
#[allow(unused)]
use gpwgpu::{parse_shaders, parse_shaders_dyn};
use macros::{UiAggregation, UiMarker};

use crate::setup::Params;

parse_shaders!(pub SHADERS, "src/shaders");
// parse_shaders_dyn!(pub SHADERS, "src/shaders");

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, UiAggregation)]
pub struct ISPParams {
    pub debayer_push: DebayerPush,
    pub black_level_push: BlackLevelPush,
    pub auto_white_balance_push: AutoWhiteBalancePush,
    pub gamma_push: GammaPush,
    pub color_correction_push: ColorCorrectionPush,
}

#[derive(Hash, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Buffers {
    Raw,
    TempMean,
    Mean,
    BlackLevel,
    AutoWhiteBalance,
    RGB,
}

pub struct PT;

pub type StateError = ShaderError;

impl PipelineTypes for PT{
    type Params = Params;

    type Buffer = Buffers;

    type Error = StateError;

    type Args = ISPParams;
}

impl Buffers {
    fn init(self, params: &Params) -> AbstractBuffer<PT> {
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
            Buffers::TempMean => AbstractBuffer {
                name,
                memory_req: MemoryReq::Temporary,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
                size: params.byte_size() as u64,
            },
            Buffers::Mean => AbstractBuffer {
                name,
                memory_req: MemoryReq::Strict,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
                size: size_of::<[f32; 4]>() as u64,
            },
            Buffers::BlackLevel => AbstractBuffer {
                name,
                memory_req: MemoryReq::Strict,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
                size: params.byte_size() as u64,
            },
            Buffers::AutoWhiteBalance => AbstractBuffer {
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
pub struct BlackLevel {
    pass: FullComputePass,
}

#[derive(
    Clone,
    Copy,
    Debug,
    bytemuck::Pod,
    bytemuck::Zeroable,
    Default,
    UiMarker,
    serde::Serialize,
    serde::Deserialize,
)]
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

impl SequentialOperation for BlackLevel {
    type PT = PT;
    // type Params = Params;
    // type BufferEnum = Buffers;
    // type Error = ShaderError;
    // type Args = ISPParams;

    fn enabled(_params: &PipelineParams<Self>) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn buffers(params: &PipelineParams<Self>) -> Vec<AbstractBuffer<PT>>
    where
        Self: Sized,
    {
        vec![Buffers::Raw.init(params), Buffers::BlackLevel.init(params)]
    }

    fn create(
        device: &gpwgpu::wgpu::Device,
        params: &PipelineParams<Self>,
        buffers: &gpwgpu::automatic_buffers::BufferSolution<PT>,
    ) -> Result<Self, PipelineError<Self>>
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

        let shader = params
            .shader_processor
            .process_by_name("black_level", specs)?;

        let pipeline = shader.build(device)?;

        let bindgroup = [(0, raw), (1, black_level)];

        let pass = FullComputePass::new(device, pipeline, &bindgroup);

        Ok(Self { pass })
    }

    fn execute(
        &mut self,
        encoder: &mut gpwgpu::utils::Encoder,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<PT>,
        args: &PipelineArgs<Self>,
    ) {
        // if !args.black_level.enabled {
        //     return;
        // }
        self.pass
            .execute(encoder, bytemuck::bytes_of(&args.black_level_push))
    }
}

#[derive(Debug)]
pub struct AutoWhiteBalance {
    align: FullComputePass,
    mean: MeanReduce,
    reduction_length: u32,

    gain_application: FullComputePass,
}

#[derive(
    Clone,
    Copy,
    Debug,
    bytemuck::Pod,
    bytemuck::Zeroable,
    Default,
    UiMarker,
    serde::Serialize,
    serde::Deserialize,
)]
#[repr(C)]
pub struct AutoWhiteBalancePush {
    pub gain: f32,
}

impl SequentialOperation for AutoWhiteBalance {
    type PT = PT;
    // type Params = Params;
    // type BufferEnum = Buffers;
    // type Error = ShaderError;
    // type Args = ISPParams;

    fn enabled(_params: &PipelineParams<Self>) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn buffers(params: &PipelineParams<Self>) -> Vec<AbstractBuffer<PT>>
    where
        Self: Sized,
    {
        vec![
            Buffers::BlackLevel.init(params),
            Buffers::TempMean.init(params),
            Buffers::Mean.init(params),
            Buffers::AutoWhiteBalance.init(params),
        ]
    }

    fn create(
        device: &gpwgpu::wgpu::Device,
        params: &PipelineParams<Self>,
        buffers: &gpwgpu::automatic_buffers::BufferSolution<PT>,
    ) -> Result<Self, PipelineError<Self>>
    where
        Self: Sized,
    {
        let black_level = buffers.get_from_any(Buffers::BlackLevel);
        let auto_white_balance = buffers.get_from_any(Buffers::AutoWhiteBalance);
        let temp_mean = buffers.get_from_any(Buffers::TempMean);
        let mean_buf = buffers.get_from_any(Buffers::Mean);

        let dispatch_size = [(params.height as u32) / 2, (params.width as u32) / 2, 1];

        let specs = ShaderSpecs::new((8, 32, 1))
            .direct_dispatcher(&dispatch_size)
            .extend_defs([
                ("HEIGHT", params.height.into()),
                ("WIDTH", params.width.into()),
            ]);

        let shader = params
            .shader_processor
            .process_by_name("bayer_to_vec4", specs)?;

        let pipeline = shader.build(device)?;

        let bindgroup = [(0, black_level), (1, temp_mean)];

        let align = FullComputePass::new(device, pipeline, &bindgroup);

        let mean = MeanReduce::new(
            device,
            temp_mean,
            None,
            None,
            mean_buf,
            8,
            ShaderSpecs::new((256, 1, 1)),
            24,
            InputType::Vec4F32,
        )?;

        let dispatch_size = [(params.height as u32), (params.width as u32), 1];
        let specs = ShaderSpecs::new((8, 32, 1))
            .direct_dispatcher(&dispatch_size)
            .extend_defs([
                ("HEIGHT", params.height.into()),
                ("WIDTH", params.width.into()),
            ]);

        let shader = params
            .shader_processor
            .process_by_name("auto_white_balance", specs)?;
        let pipeline = shader.build(device)?;

        let bindgroup = [(0, black_level), (1, auto_white_balance), (2, mean_buf)];
        let gain_application = FullComputePass::new(device, pipeline, &bindgroup);

        let reduction_length = ((params.height * params.width) / 4) as u32;

        Ok(Self {
            align,
            mean,
            reduction_length,
            gain_application,
        })
    }

    fn execute(
        &mut self,
        encoder: &mut gpwgpu::utils::Encoder,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<PT>,
        args: &PipelineArgs<Self>,
    ) {
        self.align.execute(encoder, &[]);
        self.mean.execute(encoder, self.reduction_length);
        self.gain_application
            .execute(encoder, bytes_of(&args.auto_white_balance_push));
    }
}

#[derive(Debug)]
pub struct Debayer {
    pass: FullComputePass,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, UiMarker)]
pub struct DebayerPush {
    pub enabled: i32,
}

impl SequentialOperation for Debayer {
    type PT = PT;
    // type Params = Params;
    // type BufferEnum = Buffers;
    // type Error = ShaderError;
    // type Args = ISPParams;

    fn enabled(_params: &PipelineParams<Self>) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn buffers(
        params: &PipelineParams<Self>,
    ) -> Vec<gpwgpu::automatic_buffers::AbstractBuffer<PT>>
    where
        Self: Sized,
    {
        vec![
            Buffers::AutoWhiteBalance.init(params),
            Buffers::RGB.init(params),
        ]
    }

    fn create(
        device: &gpwgpu::wgpu::Device,
        params: &PipelineParams<Self>,
        buffers: &gpwgpu::automatic_buffers::BufferSolution<PT>,
    ) -> Result<Self, PipelineError<Self>>
    where
        Self: Sized,
    {
        let bayered = buffers.get::<Self>(Buffers::AutoWhiteBalance);
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

        let pipeline = shader.build(device)?;

        let bindgroup = [(0, bayered), (1, debayered)];

        let pass = FullComputePass::new(device, pipeline, &bindgroup);

        Ok(Self { pass })
    }

    fn execute(
        &mut self,
        encoder: &mut gpwgpu::utils::Encoder,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<PT>,
        args: &PipelineArgs<Self>,
    ) {
        // let push = if args.debayer.enabled { 1i32 } else { 0 };
        self.pass
            .execute(encoder, bytes_of(&args.debayer_push.enabled));
    }
}

#[derive(Debug)]
pub struct RGBSpaceOperations {
    pass: FullComputePass,
}

#[derive(
    Clone,
    Copy,
    Debug,
    bytemuck::Pod,
    bytemuck::Zeroable,
    Default,
    UiMarker,
    serde::Serialize,
    serde::Deserialize,
)]
#[repr(C)]
pub struct ColorCorrectionPush {
    pub color_correction_matrix: glam::Mat4,
}

#[derive(
    Clone,
    Copy,
    Debug,
    bytemuck::Pod,
    bytemuck::Zeroable,
    Default,
    UiMarker,
    serde::Serialize,
    serde::Deserialize,
)]
#[repr(C)]
pub struct GammaPush {
    pub gain: f32,
    pub gamma: f32,
}

impl SequentialOperation for RGBSpaceOperations {
    type PT = PT;
    // type Params = Params;
    // type BufferEnum = Buffers;
    // type Error = ShaderError;
    // type Args = ISPParams;

    fn enabled(_params: &PipelineParams<Self>) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn buffers(params: &PipelineParams<Self>) -> Vec<AbstractBuffer<PT>>
    where
        Self: Sized,
    {
        vec![Buffers::RGB.init(params)]
    }

    fn create(
        device: &gpwgpu::wgpu::Device,
        params: &PipelineParams<Self>,
        buffers: &gpwgpu::automatic_buffers::BufferSolution<PT>,
    ) -> Result<Self, PipelineError<Self>>
    where
        Self: Sized,
    {
        let rgb = buffers.get::<Self>(Buffers::RGB);

        let dispatch_size = [params.height as u32, params.width as u32, 1];

        let specs = ShaderSpecs::new((8, 32, 1))
            .direct_dispatcher(&dispatch_size)
            .extend_defs([
                ("HEIGHT", params.height.into()),
                ("WIDTH", params.width.into()),
            ])
            .push_constants(100);

        let shader = params.shader_processor.process_by_name("rgb_space", specs)?;

        let pipeline = shader.build(device)?;

        let bindgroup = [(0, rgb)];

        let pass = FullComputePass::new(device, pipeline, &bindgroup);

        Ok(Self { pass })
    }

    fn execute(
        &mut self,
        encoder: &mut gpwgpu::utils::Encoder,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<PT>,
        args: &PipelineArgs<Self>,
    ) {
        let mut push = bytes_of(&args.color_correction_push).to_vec();
        push.extend_from_slice(bytes_of(&args.gamma_push));
        self.pass.execute(encoder, &push);
    }
}

#[derive(Debug)]
pub struct PreserveRaw;

impl SequentialOperation for PreserveRaw {
    type PT = PT;
    // type Params = Params;
    // type BufferEnum = Buffers;
    // type Error = ShaderError;
    // type Args = ISPParams;

    fn enabled(_params: &PipelineParams<Self>) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn buffers(params: &PipelineParams<Self>) -> Vec<AbstractBuffer<PT>>
    where
        Self: Sized,
    {
        vec![Buffers::Raw.init(params)]
    }

    fn create(
        _device: &gpwgpu::wgpu::Device,
        _params: &PipelineParams<Self>,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<PT>,
    ) -> Result<Self, PipelineError<Self>>
    where
        Self: Sized,
    {
        Ok(Self)
    }

    fn execute(
        &mut self,
        _encoder: &mut gpwgpu::utils::Encoder,
        _buffers: &gpwgpu::automatic_buffers::BufferSolution<PT>,
        _args: &PipelineArgs<Self>,
    ) {
    }
}

pub fn create_to_texture(
    device: &Device,
    params: &Params,
    final_buffer: &Buffer,
    texture: &Texture,
) -> Result<FullComputePass, ShaderError> {
    let dispatch_size = [params.height as u32, params.width as u32, 1];

    let shader = params.shader_processor.process_by_name(
        "to_texture",
        ShaderSpecs::new((8, 32, 1))
            .extend_defs([
                ("HEIGHT", (params.height).into()),
                ("WIDTH", (params.width).into()),
            ])
            .direct_dispatcher(&dispatch_size),
    )?;

    let pipeline = shader.build(device)?;

    let view = &texture.create_view(&Default::default());

    let bindgroup = [
        BindGroupEntry {
            binding: 0,
            resource: final_buffer.as_entire_binding(),
        },
        BindGroupEntry {
            binding: 1,
            resource: gpwgpu::wgpu::BindingResource::TextureView(&view),
        },
    ];

    let pass = FullComputePass::new(device, pipeline, &bindgroup);

    Ok(pass)
}
