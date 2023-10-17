use std::ops::Deref;

use bevy::{prelude::*, render::{RenderPlugin, settings::WgpuSettings, texture::{BevyDefault, TextureFormatPixelInfo}, renderer::{RenderDevice, RenderQueue}}, sprite::MaterialMesh2dBundle};
use gpwgpu::{wgpu, shaderpreprocessor::ShaderProcessor, utils::DebugEncoder};
// use gpwgpu::{wgpu::{self, TextureDescriptor, TextureUsages}, shaderpreprocessor::ShaderProcessor};
use viewer::{simple_renderer::{SimpleRendererPlugin, StateImage, SendState, ImageSettings}, camera2d::{My2dCameraPlugin, My2dController}};
use wgpu_isp::{setup::{Params, ISPParams, make_debug_bundle}, operations::{DebayerParams, BlackLevelParams, BlackLevelPush}};
// use wgpu_isp::operations::{BlackLevelParams, BlackLevelPush};

pub fn device_descriptor() -> wgpu::DeviceDescriptor<'static>{
    let mut desc = wgpu::DeviceDescriptor::default();
    desc.features = wgpu::Features::MAPPABLE_PRIMARY_BUFFERS | wgpu::Features::PUSH_CONSTANTS;
    desc.limits.max_push_constant_size = 64;
    desc.limits.max_storage_buffers_per_shader_stage = 12;
    return desc;
}


fn main() {
    let default_plugins = DefaultPlugins
        .build()
        .set({
            let device_descriptor = device_descriptor();
            RenderPlugin {
                wgpu_settings: WgpuSettings {
                    features: device_descriptor.features,
                    limits: device_descriptor.limits,
                    ..Default::default()
                },
            }
        });
    App::new()
        .add_plugins((
            default_plugins,
            SimpleRendererPlugin,
            My2dCameraPlugin,
        ))
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let data = std::fs::read("../tests/test.RAW").unwrap();

    let data = data
        .chunks(2)
        .map(|chunk| u16::from_ne_bytes(chunk.try_into().unwrap()) as f32)
        .collect::<Vec<_>>();


    let device = unsafe { std::mem::transmute::<&wgpu::Device, &wgpu::Device>(device.wgpu_device()) };

    let queue = unsafe { std::mem::transmute::<&wgpu::Queue, &wgpu::Queue>(queue.deref()) };

    let params = Params{
        width: 1920,
        height: 1080,
        shader_processor: ShaderProcessor::load_dir_dyn("../src/shaders").unwrap(),
    };

    let state = wgpu_isp::setup::State::new(device, queue, params).unwrap();

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

    state.write_to_input(&data);

    let mut encoder = DebugEncoder::new(&state.device);

    state.sequential.execute(&mut encoder, &isp_params);

    // encoder.set_debug_bundle(make_debug_bundle(&state));
    // encoder.activate();
    // encoder.inspect_buffers().unwrap();
    
    state.to_texture.execute(&mut encoder, &[]);

    encoder.submit(state.queue);
    
    let state_image = StateImage::new(state);


    let image_settings = ImageSettings{
        size: Vec2::new(1920., 1080.),
        // size: Vec2::new(1080., 1920.),
        anchor: Vec2::splat(0.0),
        flip_x: true,
        flip_y: true,
    };
    
    commands.spawn(Camera2dBundle::default())
        .insert(My2dController::default());

    let scale = 0.5;

    commands.spawn((state_image, SpatialBundle{
        transform: Transform::from_scale(Vec3::splat(scale)),
        ..default()
    }, image_settings));
    

}
