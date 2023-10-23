use std::{ops::Deref, time::Duration, path::Path};

use bevy::{
    prelude::*,
    render::{
        renderer::{RenderDevice, RenderQueue},
        settings::WgpuSettings,
        RenderPlugin,
    },
};
use bevy_egui::{
    egui::{self, CollapsingHeader, Slider, Ui},
    EguiContexts, EguiPlugin,
};
use gpwgpu::{shaderpreprocessor::ShaderProcessor, utils::DebugEncoder, wgpu, FutureExt};
use macros::generate_ui_impl;
use notify::{RecursiveMode, Watcher};
use viewer::{
    camera2d::{My2dCameraPlugin, My2dController},
    file_watcher::FilesystemWatcher,
    simple_renderer::{ImageSettings, SimpleRendererPlugin, StateImage},
    ui_form::{BoundedSlider, IntCheckbox, Mat4Slider},
};
use wgpu_isp::{
    operations::{
        AutoWhiteBalancePush, BlackLevelPush, ColorCorrectionPush, DebayerPush, GammaPush,
        ISPParams,
    },
    setup::Params,
};

pub fn device_descriptor() -> wgpu::DeviceDescriptor<'static> {
    let mut desc = wgpu::DeviceDescriptor::default();
    desc.features = wgpu::Features::MAPPABLE_PRIMARY_BUFFERS | wgpu::Features::PUSH_CONSTANTS;
    desc.limits.max_push_constant_size = 100;
    desc.limits.max_storage_buffers_per_shader_stage = 12;
    return desc;
}

// Autogenerates some UI based on the operations we are using.
// This is generated based on the structs annotated with derive(UiMarker)
// in the file that we pass to the macro.
//
// A bunch of structs and impls are output in the current scope.
// The top level is FullUi, which can be shown using the struct marked as
// UiAggregation (here ISPParams)
generate_ui_impl! {"src/operations.rs"}

#[derive(Component)]
struct UiComponent {
    in_out_json: (String, Option<ErrString>),
    full_ui: FullUi,
}

fn main() {
    let default_plugins = DefaultPlugins.build().set({
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
            EguiPlugin,
        ))
        .add_systems(Startup, setup_scene)
        .init_resource::<ThisFileWatcher>()
        .add_systems(Update, (re_execute, ui, watch_for_shader_changes))
        .run();
}

#[derive(Resource)]
struct ThisFileWatcher(FilesystemWatcher);

impl Default for ThisFileWatcher {
    fn default() -> Self {
        let mut watcher = FilesystemWatcher::new(Duration::from_millis(50));

        watcher
            .watcher
            .watch("../src/shaders".as_ref(), RecursiveMode::Recursive)
            .unwrap();

        Self(watcher)
    }
}

fn watch_for_shader_changes(
    watcher: Res<ThisFileWatcher>,
    mut query: Query<(&mut ShouldExecute, &mut StateImage)>,
) {
    if let Ok(_event) = watcher.0.receiver.try_recv() {
        for (mut should_execute, mut state) in &mut query {
            let shader_processor = match ShaderProcessor::load_dir_dyn("../src/shaders") {
                Ok(processor) => processor,
                Err(e) => {
                    dbg!(e);
                    continue;
                }
            };
            let params = Params {
                shader_processor,
                ..state.state.0.params
            };

            // state
            //     .state
            //     .0
            //     .device
            //     .push_error_scope(wgpu::ErrorFilter::Validation);
            let new_state = match state.state.0.reload(params) {
                Ok(state) => state,
                Err(e) => {
                    dbg!(e);
                    continue;
                }
            };
            // if let Some(err) = state.state.0.device.pop_error_scope().block_on() {
            //     println!("[{}:{}]\n{}", file!(), line!(), err);
            //     continue;
            // }
            let mut encoder = new_state.device.create_command_encoder(&default());

            let old_input = state
                .state
                .0
                .sequential
                .buffers
                .get_from_any(wgpu_isp::operations::Buffers::Raw);

            encoder.copy_buffer_to_buffer(
                old_input,
                0,
                new_state
                    .sequential
                    .buffers
                    .get_from_any(wgpu_isp::operations::Buffers::Raw),
                0,
                old_input.size(),
            );

            new_state.queue.submit(Some(encoder.finish()));

            *state = StateImage::new(new_state);
            should_execute.0 = true;
        }
    }
}

#[derive(Component)]
struct ParamsComponent(ISPParams);

fn setup_scene(mut commands: Commands, device: Res<RenderDevice>, queue: Res<RenderQueue>) {
    let data = std::fs::read("../tests/test.RAW").unwrap();

    let data = data
        .chunks(2)
        .map(|chunk| u16::from_ne_bytes(chunk.try_into().unwrap()) as f32)
        .collect::<Vec<_>>();

    let device =
        unsafe { std::mem::transmute::<&wgpu::Device, &wgpu::Device>(device.wgpu_device()) };

    let queue = unsafe { std::mem::transmute::<&wgpu::Queue, &wgpu::Queue>(queue.deref()) };

    let params = Params {
        width: 1920,
        height: 1080,
        shader_processor: ShaderProcessor::load_dir_dyn("../src/shaders").unwrap(),
    };

    let image_settings = ImageSettings {
        size: Vec2::new(params.width as f32, params.height as f32),
        anchor: Vec2::splat(0.0),
        flip_x: true,
        flip_y: true,
    };

    let state = wgpu_isp::setup::State::new(device, queue, params).unwrap();

    let isp_params = ISPParams {
        debayer_push: DebayerPush { enabled: 1 },
        black_level_push: BlackLevelPush::default(),
        auto_white_balance_push: AutoWhiteBalancePush { gain: 1.0 },
        gamma_push: GammaPush {
            gain: 1.0,
            gamma: 1.0,
        },
        color_correction_push: ColorCorrectionPush {
            color_correction_matrix: Mat4::IDENTITY,
        },
    };

    state.write_to_input(&data);

    let state_image = StateImage::new(state);

    commands
        .spawn(Camera2dBundle::default())
        .insert(My2dController::default());

    let scale = 0.5;

    let mut counter = 0;

    commands.spawn((
        state_image,
        SpatialBundle {
            transform: Transform::from_scale(Vec3::splat(scale)),
            ..default()
        },
        image_settings,
        ShouldExecute(true),
        ParamsComponent(isp_params),
        UiComponent {
            full_ui: FullUi::new(|| {
                let cur = counter;
                counter += 1;
                cur
            }),
            in_out_json: (String::new(), None),
        },
    ));
}

#[derive(Component)]
struct ShouldExecute(bool);

fn re_execute(mut query: Query<(&ParamsComponent, &mut ShouldExecute, &StateImage)>) {
    for (params, mut should_execute, state) in &mut query {
        if !should_execute.0 {
            continue;
        }
        let state = &state.state.0;

        let mut encoder = DebugEncoder::new(&state.device);

        state.sequential.execute(&mut encoder, &params.0);

        state.to_texture.execute(&mut encoder, &[]);

        encoder.submit(&state.queue);

        should_execute.0 = false;
    }
}

struct ErrString(String);

impl Deref for ErrString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ToString> From<T> for ErrString{
    fn from(value: T) -> Self {
        Self(value.to_string())
    }
}

fn load_json(path: impl AsRef<Path>) -> Result<ISPParams, ErrString>{
    let contents = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str::<ISPParams>(&contents)?)
}

fn save_json(path: impl AsRef<Path>, params: &ISPParams) -> Result<(), ErrString>{
    let s = serde_json::to_string_pretty(params)?;
    std::fs::write(path, s)?;
    Ok(())
}

fn ui(
    mut egui_contexts: EguiContexts,
    mut query: Query<(&mut ParamsComponent, &mut ShouldExecute, &mut UiComponent)>,
) {
    let ctx = egui_contexts.ctx_mut();

    egui::SidePanel::left("primary_panel").show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (mut params, mut should_execute, mut ui_state) in &mut query {
                ui.label("Load or save parameters to json");
                ui.text_edit_singleline(&mut ui_state.in_out_json.0);
                if let Some(err) = &ui_state.in_out_json.1{
                    ui.label(format!("{}", err.deref()));
                }
                ui.horizontal(|ui|{
                    if ui.button("Load").clicked(){
                        match load_json(&ui_state.in_out_json.0){
                            Ok(loaded_params) => {
                                params.0 = loaded_params;
                                ui_state.in_out_json.1 = None;
                            },
                            Err(err_str) => {
                                ui_state.in_out_json.1 = Some(err_str)
                            }
                        }
                    }
                    if ui.button("Save").clicked(){
                        match save_json(&ui_state.in_out_json.0, &params.0){
                            Ok(()) => {
                                ui_state.in_out_json.1 = None;
                            },
                            Err(err_str) => {
                                ui_state.in_out_json.1 = Some(err_str)
                            }
                        }
                    }
                });
                should_execute.0 = ui_state.full_ui.show(ui, &mut params.0);
            }
        })
    });
}
