use gpwgpu::{
    bytemuck,
    utils::{default_device, DebugEncoder},
    FutureExt,
};
use std::time::Instant;
use wgpu_isp::{
    operations::{BlackLevelParams, BlackLevelPush, Buffers, DebayerParams, SHADERS},
    setup::{ISPParams, Params, State},
};

#[allow(unused)]
use wgpu_isp::setup::make_debug_bundle;


#[test]
fn runner() {
    let (device, queue) = default_device().block_on().unwrap();

    let params = Params {
        width: 1920,
        height: 1080,
        shader_processor: SHADERS.clone(),
    };

    let isp_params = ISPParams {
        debayer: DebayerParams { enabled: true },
        black_level: BlackLevelParams {
            enabled: true,
            push: BlackLevelPush {
                r_offset: 0.0,
                gr_offset: 0.0,
                gb_offset: 0.0,
                b_offset: 0.0,
                alpha: 0.0,
                beta: 0.0,
            },
        },
    };

    let state = State::new(&device, &queue, params).unwrap();

    let data = std::fs::read("tests/test.RAW").unwrap();

    let data = data
        .chunks(2)
        .map(|chunk| u16::from_ne_bytes(chunk.try_into().unwrap()) as f32)
        .collect::<Vec<_>>();

    let input_buf = state.sequential.buffers.get_from_any(Buffers::Raw);

    let now = Instant::now();
    for _ in 0..1000 {
        queue.write_buffer(input_buf, 0, bytemuck::cast_slice(&data));

        let mut encoder = DebugEncoder::new(&device);
        // encoder.set_debug_bundle(make_debug_bundle(&state));
        state.sequential.execute(&mut encoder, &isp_params);

        // encoder.activate();
        // encoder.inspect_buffers().unwrap();

        device.poll(gpwgpu::wgpu::MaintainBase::Wait);
        encoder.submit(&queue);
    }
    dbg!(now.elapsed());

    let mut encoder = DebugEncoder::new(&device);
    state.to_texture.execute(&mut encoder, &[]);
    encoder.submit(&state.queue);
    

    // let retrieved = read_buffer::<f32>(&device, output_buf, 0, None);
}
