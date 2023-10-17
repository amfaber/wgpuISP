use bevy::{
    input::{
        mouse::{MouseScrollUnit, MouseWheel},
        touchpad::TouchpadMagnify,
    },
    prelude::*,
    render::camera::CameraProjection, window::PrimaryWindow,
};

pub fn viewport_position(
    camera: &Camera,
    window: &Window,
) -> Option<Vec2>{
    let position = window.cursor_position()?;
    let position = match camera.logical_viewport_rect(){

        Some(Rect { min, max }) => {
            let position = position - min;
            if position.cmpge(Vec2::ZERO).all() && position.cmple(max).all(){
                Some(position)
            } else {
                None
            }
        },
        None => Some(position),
    };
    position
}

#[derive(Default)]
pub struct My2dCameraPlugin;

impl Plugin for My2dCameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Last, camera_control_2d);
    }
}

#[derive(Component)]
pub struct My2dController {
    pub enabled: bool,
    pub pan_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub pixels_per_line: f32,
    pub pinch_sensitivity: f32,

    // Cursor world position, camera position
    pub clicked_position: Option<Vec2>,
}

impl Default for My2dController {
    fn default() -> Self {
        Self {
            enabled: true,
            zoom_sensitivity: 0.01,
            pan_sensitivity: 500.,
            pixels_per_line: 53.,   // ??
            pinch_sensitivity: 50., // ??
            clicked_position: None,
        }
    }
}

#[derive(Component)]
struct Old;

#[derive(Component)]
struct New;

fn cursor_to_world(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    window: &Window,
) -> Option<Vec2> {
    let Some(pos) = viewport_position(camera, window) else { return None };
    let Some(world) = camera.viewport_to_world_2d(camera_transform, pos) else { return None };

    Some(world)
}

fn move_camera_to_fix_point(
    camera_transform: &mut Transform,
    projection: &mut OrthographicProjection,
    old_world: Vec2,
    camera: &Camera,
    window: &Window,
) -> Option<()> {
    if let Some(size) = camera.logical_viewport_size(){
        projection.update(size.x, size.y);
    }
    let new_viewport = viewport_position(camera, window)?;
    let new_ndc = viewport_to_ndc(camera, new_viewport)?;

    let pinv = projection.get_projection_matrix().as_dmat4().inverse();
    let a2 = pinv.x_axis[0];
    let b2 = pinv.y_axis[1];

    camera_transform.translation.x = (old_world.x as f64 - a2 * new_ndc.x as f64) as f32;

    camera_transform.translation.y = (old_world.y as f64 - b2 * new_ndc.y as f64) as f32;
    Some(())
}

fn camera_control_2d(
    keyboard: Res<Input<KeyCode>>,
    mut mouse_wheel_reader: EventReader<MouseWheel>,
    mut touchpad_magnify: EventReader<TouchpadMagnify>,
    mut controllers: Query<
        (
            &Camera,
            &GlobalTransform,
            &mut My2dController,
            &mut Transform,
            &mut OrthographicProjection,
        ),
        (Without<Old>, Without<New>),
    >,
    time: Res<Time>,
    mouse_input: Res<Input<MouseButton>>,

	window: Query<&Window, With<PrimaryWindow>>,
) {
    let delta_time = time.delta_seconds();

    let shift_pressed =
        keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

	let window = window.single();
	

    for (camera, global, mut controller, mut transform, mut projection) in
        controllers.iter_mut()
    {
        if !controller.enabled {
            continue;
        }

        if !window.focused {
            continue;
        }

        if mouse_input.just_pressed(MouseButton::Left) && (shift_pressed) {
            controller.clicked_position = cursor_to_world(camera, global, window);
        }
        if mouse_input.just_released(MouseButton::Left) {
            controller.clicked_position = None
        }

        let mut locomotion_dir = Vec3::ZERO;

        let mut wheel_delta = 0.0;

        if shift_pressed {
            for key in keyboard.get_pressed() {
                match key {
                    KeyCode::A => {
                        locomotion_dir.x -= 1.0;
                    }
                    KeyCode::D => {
                        locomotion_dir.x += 1.0;
                    }
                    KeyCode::S => {
                        locomotion_dir.y -= 1.0;
                    }
                    KeyCode::W => {
                        locomotion_dir.y += 1.0;
                    }

                    KeyCode::E => {
                        wheel_delta += 1.0;
                    }
                    KeyCode::Q => {
                        wheel_delta -= 1.0;
                    }
                    _ => {}
                }
            }
        }

        for event in mouse_wheel_reader.iter() {
            wheel_delta += match event.unit {
                MouseScrollUnit::Line => event.y,
                MouseScrollUnit::Pixel => event.y / controller.pixels_per_line,
            };
        }

        for event in touchpad_magnify.iter() {
            wheel_delta += event.0 * controller.pinch_sensitivity;
        }

        if wheel_delta != 0. {
            let old_world = cursor_to_world(camera, global, window);
            projection.scale -=
                wheel_delta * controller.zoom_sensitivity * projection.scale.min(1.);
            projection.scale = projection.scale.max(0.01);

            if let Some(old_world) = old_world{
                move_camera_to_fix_point(&mut transform, &mut projection, old_world, camera, window);
            }
        }

        transform.translation +=
            controller.pan_sensitivity * locomotion_dir * delta_time * projection.scale;

        if let Some(old_world) = controller.clicked_position {
            move_camera_to_fix_point(&mut transform, &mut projection, old_world, camera, window);
        }
    }
}

fn viewport_to_ndc(camera: &Camera, mut viewport_position: Vec2) -> Option<Vec3> {
    let target_size = camera.logical_viewport_size()?;
    // Flip the Y co-ordinate origin from the top to the bottom .
    viewport_position.y = target_size.y - viewport_position.y;
    let ndc = viewport_position * 2. / target_size - Vec2::ONE;
    let ndc = ndc.extend(1.);
    Some(ndc)
}

