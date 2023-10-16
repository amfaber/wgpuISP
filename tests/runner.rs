use gpwgpu::{
    bytemuck,
    utils::{default_device, DebugBundle, DebugEncoder, InspectBuffer},
    FutureExt,
};
use std::{marker::PhantomData, time::Instant};
use wgpu_isp::{
    operations::{BlackLevelParams, BlackLevelPush, Buffers, DebayerParams},
    setup::{ISPParams, Params, State},
};

#[test]
fn runner() {
    let (device, queue) = default_device().block_on().unwrap();

    let params = Params {
        width: 1920,
        height: 1080,
        isp: ISPParams {
            debayer: DebayerParams { enabled: true },
            black_level: BlackLevelParams {
                enabled: true,
                push: BlackLevelPush {
                    r_offset: 0.0,
                    gr_offset: 0.,
                    gb_offset: 0.0,
                    b_offset: 0.0,
                    alpha: 0.0,
                    beta: 0.0,
                },
            },
        },
        phan: PhantomData::<u16>,
    };

    let state = State::new(&device, &queue, params).unwrap();

    let data = std::fs::read("tests/test.RAW").unwrap();

    let data = data
        .chunks(2)
        .map(|chunk| u16::from_ne_bytes(chunk.try_into().unwrap()) as f32)
        .collect::<Vec<_>>();

    let input_buf = state.sequential.buffers.get_from_any(Buffers::Raw);

    let output_buf = state.sequential.buffers.get_from_any(Buffers::RGB);

    let now = Instant::now();
    for _ in 0..1000 {
        queue.write_buffer(input_buf, 0, bytemuck::cast_slice(&data));

        let mut encoder = DebugEncoder::new(&device);
        encoder.set_debug_bundle(DebugBundle {
            device: &device,
            queue: &queue,
            inspects: vec![
                InspectBuffer::new(input_buf, None, "input"),
                InspectBuffer::new(
                    state.sequential.buffers.get_from_any(Buffers::BlackLevel),
                    None,
                    "black_level",
                ),
                InspectBuffer::new(output_buf, None, "output"),
            ],
            save_path: "tests/dumps".into(),
            create_py: true,
        });
        state.sequential.execute(&mut encoder, &state.params);

        // encoder.activate();
        // encoder.inspect_buffers().unwrap();

        device.poll(gpwgpu::wgpu::MaintainBase::Wait);
        encoder.submit(&queue);
    }
    dbg!(now.elapsed());

    // let retrieved = read_buffer::<f32>(&device, output_buf, 0, None);
}
