
use std::{marker::PhantomData, time::Instant};
use gpwgpu::{FutureExt, utils::{default_device, DebugEncoder, DebugBundle, InspectBuffer}, bytemuck};
use wgpu_isp::{setup::{State, Params}, operations::Buffers};

#[test]
fn runner() {
    let (device, queue) = default_device().block_on().unwrap();

    let params = Params{
        width: 1920,
        height: 1080,
        phan: PhantomData::<u16>,
    };

    let state = State::new(&device, &queue, params).unwrap();
	
	let data = std::fs::read("tests/test.RAW").unwrap();

	let data = data.chunks(2).map(|chunk|{
		u16::from_ne_bytes(chunk.try_into().unwrap()) as f32
	}).collect::<Vec<_>>();
	
	let input_buf = state.sequential.buffers.get_from_any(Buffers::Bayered);

	let output_buf = state.sequential.buffers.get_from_any(Buffers::Debayered);

	let now = Instant::now();
	for _ in 0..100{
		queue.write_buffer(input_buf, 0, bytemuck::cast_slice(&data));

		let mut encoder = DebugEncoder::new(&device);
		encoder.set_debug_bundle(DebugBundle{
		    device: &device,
		    queue: &queue,
		    inspects: vec![
				InspectBuffer::new(input_buf, None, "input"),
				InspectBuffer::new(output_buf, None, "output"),
			],
		    save_path: "tests/dumps".into(),
		    create_py: true,
		});
		// encoder.activate();
  //       encoder.inspect_buffers().unwrap();

		state.sequential.execute(&mut encoder, &());

		device.poll(gpwgpu::wgpu::MaintainBase::Wait);
		encoder.submit(&queue);
	}
	dbg!(now.elapsed());

	

	// let retrieved = read_buffer::<f32>(&device, output_buf, 0, None);

	
	
}
