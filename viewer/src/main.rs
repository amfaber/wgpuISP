use std::{mem::size_of, ops::Deref, path::Path, str::FromStr, time::Duration};

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    render::{
        renderer::{RenderDevice, RenderQueue},
        settings::WgpuSettings,
        RenderPlugin,
    },
};
use bevy_egui::{
    egui::{self, CollapsingHeader, TextEdit, Ui, Widget, Response},
    EguiContexts, EguiPlugin,
};
use bytemuck::cast_slice;
use gpwgpu::{shaderpreprocessor::ShaderProcessor, utils::DebugEncoder, wgpu};
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
        AutoWhiteBalancePush, BlackLevelPush, Buffers, ColorCorrectionPush, DebayerPush, GammaPush,
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

#[derive(Default)]
struct Field {
    content: String,
    err: Option<ErrString>,
    id: usize,
}

impl Field {
    fn parse<'a: 'b, 'b, T, E: std::fmt::Display>(
        &'a mut self,
        f: impl Fn(&'b str) -> Result<T, E>,
    ) -> Option<T> {
        match f(&self.content) {
            Ok(parsed) => {
                self.err = None;
                Some(parsed)
            }
            Err(err) => {
                self.err = Some(err.into());
                None
            }
        }
    }

    fn single_line(&mut self, ui: &mut Ui) -> Response{
        TextEdit::singleline(&mut self.content).id_source(self.id).ui(ui)
    }
}

#[derive(Default)]
struct InputUiState {
    file: Field,
    width: Field,
    height: Field,
}

#[derive(Component)]
struct UiComponent {
    file_input: InputUiState,
    in_out_json: Field,
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
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin,
        ))
        .add_systems(Startup, setup_scene)
        .init_resource::<ThisFileWatcher>()
        .add_systems(
            Update,
            (re_execute, ui, watch_for_shader_changes, new_input),
        )
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

            let new_state = match state.state.0.reload(params) {
                Ok(state) => state,
                Err(e) => {
                    dbg!(e);
                    continue;
                }
            };
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

fn setup_scene(mut commands: Commands) {
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

    commands
        .spawn(Camera2dBundle::default())
        .insert(My2dController::default());

    let scale = 0.5;

    let mut counter = 0;

    let mut id_provider = || {
        let cur = counter;
        counter += 1;
        cur
    };

    commands.spawn((
        FrameChange::NewInput,
        SpatialBundle {
            transform: Transform::from_scale(Vec3::splat(scale)),
            ..default()
        },
        ParamsComponent(isp_params),
        UiComponent {
            full_ui: FullUi::new(&mut id_provider),
            in_out_json: Field::default(),
            file_input: InputUiState {
                file: Field {
                    content: r"C:\Users\andre\Downloads\MPV-cam1-left.raw".to_string(),
                    err: None,
                    id: id_provider(),
                },
                width: Field {
                    content: "3840".to_string(),
                    err: None,
                    id: id_provider(),
                },
                height: Field {
                    content: "2160".to_string(),
                    err: None,
                    id: id_provider(),
                },
            },
        },
    ));
}

#[derive(Component)]
struct ShouldExecute(bool);

/// This rebuilds the state
// #[derive(Component)]
// struct NewInput(bool);

#[derive(Component, Clone, Copy)]
enum FrameChange {
    NotRequired,
    NewInput,
    Reload,
}

/// This uses the same state, but uploads a new image to the GPU
// #[derive(Component)]
// struct NewFrame(bool);

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

impl<T: ToString> From<T> for ErrString {
    fn from(value: T) -> Self {
        Self(value.to_string())
    }
}

fn load_json(path: impl AsRef<Path>) -> Result<ISPParams, ErrString> {
    let contents = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str::<ISPParams>(&contents)?)
}

fn save_json(path: impl AsRef<Path>, params: &ISPParams) -> Result<(), ErrString> {
    let s = serde_json::to_string_pretty(params)?;
    std::fs::write(path, s)?;
    Ok(())
}

fn json_line(
    ui: &mut Ui,
    ui_state: &mut Mut<UiComponent>,
    params: &mut Mut<ParamsComponent>,
    should_execute: &mut Mut<ShouldExecute>,
) {
    ui.label("Load or save parameters to json");

    ui.text_edit_singleline(&mut ui_state.in_out_json.content);
    if let Some(err) = &ui_state.in_out_json.err {
        ui.label(format!("{}", err.deref()));
    }
    ui.horizontal(|ui| {
        if ui.button("Load").clicked() {
            match load_json(&ui_state.in_out_json.content) {
                Ok(loaded_params) => {
                    params.0 = loaded_params;
                    should_execute.0 |= true;
                    ui_state.in_out_json.err = None;
                }
                Err(err_str) => ui_state.in_out_json.err = Some(err_str),
            }
        }
        if ui.button("Save").clicked() {
            match save_json(&ui_state.in_out_json.content, &params.0) {
                Ok(()) => {
                    ui_state.in_out_json.err = None;
                }
                Err(err_str) => ui_state.in_out_json.err = Some(err_str),
            }
        }
    });
}

fn input_line(
    ui: &mut Ui,
    // new_input: &mut Mut<NewInput>,
    new_input: &mut Mut<FrameChange>,
    ui_state: &mut Mut<UiComponent>,
    // state_image: &mut Mut<StateImage>,
) {
    ui.label("Enter a file input:");

    if ui_state.file_input.file.single_line(ui).changed(){
        **new_input = FrameChange::NewInput;
    }
    if let Some(err) = &ui_state.file_input.file.err {
        ui.label(&err.0);
    }

    if ui_state.file_input.width.single_line(ui).changed(){
        **new_input = FrameChange::NewInput;
    }
    if let Some(err) = &ui_state.file_input.width.err {
        ui.label(&err.0);
    }

    if ui_state.file_input.height.single_line(ui).changed(){
        **new_input = FrameChange::NewInput;
    }
    if let Some(err) = &ui_state.file_input.height.err {
        ui.label(&err.0);
    }
}

fn new_input(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut FrameChange,
        &mut UiComponent,
        Option<&mut StateImage>,
        Option<&mut ShouldExecute>,
    )>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    for (entity, mut new_input, mut ui_component, state_image, mut should_execute) in &mut query {
        if matches!(*new_input, FrameChange::NotRequired) {
            continue;
        }

        match *new_input {
            FrameChange::NotRequired => continue,
            FrameChange::NewInput => {
                let data = ui_component.file_input.file.parse(std::fs::read);
                let width = ui_component
                    .file_input
                    .width
                    .parse(<i32 as FromStr>::from_str);

                let height = ui_component
                    .file_input
                    .height
                    .parse(<i32 as FromStr>::from_str);

                let (Some(data), Some(width), Some(height)) = (data, width, height) else {
                    continue;
                };

                if (width * height * size_of::<u16>() as i32) != data.len() as i32 {
                    ui_component.file_input.file.err =
                        Some(ErrString("File size doesn't match dimensions".into()));
                    continue;
                }

                let shader_processor = match ShaderProcessor::load_dir_dyn("../src/shaders") {
                    Ok(processor) => processor,
                    Err(e) => {
                        dbg!(e);
                        continue;
                    }
                };

                let params = Params {
                    width,
                    height,
                    shader_processor,
                };

                let image_settings = ImageSettings {
                    size: Vec2::new(params.width as f32, params.height as f32),
                    anchor: Vec2::splat(0.0),
                    flip_x: false,
                    flip_y: false,
                };

                let device = unsafe {
                    std::mem::transmute::<&wgpu::Device, &wgpu::Device>(device.wgpu_device())
                };

                let queue =
                    unsafe { std::mem::transmute::<&wgpu::Queue, &wgpu::Queue>(queue.deref()) };

                let state = wgpu_isp::setup::State::new(device, queue, params).unwrap();

                let data = data
                    .chunks(2)
                    .map(|chunk| u16::from_ne_bytes(chunk.try_into().unwrap()) as f32)
                    .collect::<Vec<_>>();

                state.write_to_input(&data);
                let mut state_image = StateImage::new(state);
                state_image.cpu_side_data = Some(data);

                commands
                    .entity(entity)
                    .insert(state_image)
                    .insert(image_settings)
                    .insert(ShouldExecute(true));
            }
            FrameChange::Reload => {
                let Some(data) = ui_component.file_input.file.parse(std::fs::read) else {
                    continue;
                };
                let data = data
                    .chunks(2)
                    .map(|chunk| u16::from_ne_bytes(chunk.try_into().unwrap()) as f32)
                    .collect::<Vec<_>>();

                should_execute.as_mut().unwrap().0 = true;

                let state = state_image.as_ref().unwrap();
                state.state.0.write_to_input(&data);
                // state.state.0.write_to_input(state.cpu_side_data.as_ref().unwrap());
            }
        }

        *new_input = FrameChange::NotRequired;
    }
}

fn ui(
    mut egui_contexts: EguiContexts,
    mut query: Query<(
        &mut ParamsComponent,
        &mut ShouldExecute,
        &mut UiComponent,
        &mut FrameChange,
    )>,
) {
    let ctx = egui_contexts.ctx_mut();

    egui::SidePanel::left("primary_panel").show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (mut params, mut should_execute, mut ui_state, mut new_input) in &mut query {
                input_line(ui, &mut new_input, &mut ui_state);
                json_line(ui, &mut ui_state, &mut params, &mut should_execute);

                should_execute.0 |= ui_state.full_ui.show(ui, &mut params.0);
            }
        })
    });
}
