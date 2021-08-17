// Partially based on https://github.com/sburris0/bevy_flycam/blob/3350f6626382694217b50a197befcce66f2bf050/src/lib.rs
// Original code is licensed under MIT, see LICENSES/bevy_flycam
use std::{collections::HashSet, ops::Div, time::Duration};

use bevy::app::{Events, ManualEventReader};
use bevy::ecs::system::SystemParam;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy_rapier3d::{
    na::UnitQuaternion,
    physics::{EventQueue, RigidBodyHandleComponent},
    rapier::{dynamics::RigidBodySet, geometry::ColliderSet, math::Vector},
};
use kurinji::{Kurinji, OnActionBegin, OnActionProgress};

use crate::world::{
    absolut_to_index_i32, compute_is_airborn, defaults, index_to_absolut,
    raycast::RaycastSelection, BlockType, EventChunkAction,
};
use crate::{config::CobbleConfig, inventory::Inventory};

/// System labels for ECS
#[derive(Clone, PartialEq, Eq, Hash, Debug, SystemLabel)]
pub enum ControllerLabels {
    PlayerMove,
    ProcessInput,
}

pub struct NoCameraPlayerPlugin;
impl Plugin for NoCameraPlayerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<InputState>()
            .init_resource::<MovementSettings>()
            .add_startup_system(init.system())
            .add_startup_system(mapping.system())
            .add_system(player_move.system().label(ControllerLabels::PlayerMove))
            .add_system(player_look.system())
            .add_system(cursor_grab.system())
            .add_system(process_input.system().label(ControllerLabels::ProcessInput));
    }
}

const SENSITIVITY_COEFF: f32 = 0.1;

/// Keeps track of mouse motion events, pitch, and yaw
#[derive(Default)]
struct InputState {
    reader_motion: ManualEventReader<MouseMotion>,
    pitch: f32,
    yaw: f32,
}

/// Mouse sensitivity and movement speed
pub struct MovementSettings {
    pub sensitivity: f32,
    pub speed: f32,
    pub fly: bool,
}

impl Default for MovementSettings {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            speed: 6.0,
            fly: false,
        }
    }
}

pub struct CameraTag;
pub struct BodyTag;
pub struct YawTag;

pub struct MovementState {
    pub airborn: bool,
    pub intersections: HashSet<(usize, u64)>,
    last_jump: Duration,
    last_grounded: Duration,
    last_airborn: Duration,
}

impl Default for MovementState {
    fn default() -> Self {
        Self {
            airborn: true,
            intersections: HashSet::with_capacity(1),
            last_jump: Duration::new(0, 0),
            last_grounded: Duration::from_secs(u64::MAX),
            last_airborn: Duration::from_secs(0),
        }
    }
}

fn toggle_grab_cursor(window: &mut Window) {
    window.set_cursor_lock_mode(!window.cursor_locked());
    window.set_cursor_visibility(!window.cursor_visible());
}

fn set_grab_cursor(window: &mut Window, value: bool) {
    window.set_cursor_lock_mode(value);
    window.set_cursor_visibility(!value);
}

fn init(
    config: Res<CobbleConfig>,
    mut settings: ResMut<MovementSettings>,
    mut windows: ResMut<Windows>,
) {
    let window = windows.get_primary_mut().unwrap();
    set_grab_cursor(window, config.input.initial_cursor_grab);
    settings.sensitivity = config.input.sensitivity;
}

#[derive(SystemParam)]
pub struct PlayerMoveParams<'a> {
    input: Res<'a, Kurinji>,
    windows: Res<'a, Windows>,
    settings: Res<'a, MovementSettings>,
    collider_set: Res<'a, ColliderSet>,
    events: Res<'a, EventQueue>,
    time: Res<'a, Time>,
}

fn player_move(
    params: PlayerMoveParams,
    mut bodies: ResMut<RigidBodySet>,
    mut input_events: EventReader<OnActionProgress>,
    query: Query<&RigidBodyHandleComponent, With<BodyTag>>,
    mut state: Local<MovementState>,
) {
    // Figure out whether the player is airborn based on a collider sensor parented to the player
    // model
    compute_is_airborn(&params.events, &params.collider_set, &mut state);
    if state.airborn {
        state.last_airborn = params.time.time_since_startup();
    } else {
        state.last_grounded = params.time.time_since_startup();
    }

    let window = params.windows.get_primary().unwrap();
    if let Ok(body_handle) = query.single() {
        let body = bodies.get_mut(body_handle.handle()).unwrap();
        body.set_gravity_scale(if params.settings.fly { 0.0 } else { 1.0 }, true);

        let mut velocity = Vec3::ZERO;
        let sprint_factor =
            if !params.settings.fly && params.input.is_action_active("MOVE_MOD_FAST") {
                1.5
            } else if !params.settings.fly && params.input.is_action_active("MOVE_MOD_SLOW_DESC") {
                0.6
            } else {
                1.0
            };
        let forward = Vector::new(0.0, 0.0, -sprint_factor);
        let right = Vector::new(0.6, 0.0, 0.0);
        #[inline(always)]
        fn as_bevy(a: Vector<f32>) -> Vec3 {
            Vec3::new(a.x, a.y, a.z)
        }
        let pos = body.position();
        let forward = as_bevy(pos.rotation.transform_vector(&forward));
        let right = as_bevy(pos.rotation.transform_vector(&right));
        let up = Vec3::new(0.0, 1.0, 0.0);

        if window.cursor_locked() {
            for event in input_events.iter() {
                match event.action.as_str() {
                    "MOVE_FORWARD" => velocity += forward,
                    "MOVE_BACKWARD" => velocity -= forward,
                    "MOVE_LEFT" => velocity -= right,
                    "MOVE_RIGHT" => velocity += right,
                    "MOVE_JUMP" => velocity += up,
                    "MOVE_MOD_SLOW_DESC" if params.settings.fly => velocity -= up,
                    _ => (),
                }
            }
        }

        #[inline(always)]
        fn airborn_speed_coefficient(x: f32) -> f32 {
            1.005_937_3 * (1.527_939_2 * x).exp()
        }
        velocity *= params.settings.speed;
        if !params.settings.fly {
            velocity /= airborn_speed_coefficient(
                (state.last_airborn.as_millis() as f32 - state.last_grounded.as_millis() as f32)
                    .div(1000.0)
                    .clamp(0.0, 1000.0),
            );
        }

        if !velocity.is_nan() && velocity.abs().max_element() > 1.0e-3 {
            if !params.settings.fly {
                if velocity.y.abs() >= f32::EPSILON
                    && params.time.time_since_startup() - state.last_jump
                        > Duration::from_millis(1000)
                    && !state.airborn
                {
                    state.last_jump = params.time.time_since_startup();
                    velocity.y = 0.0;
                    body.apply_force(Vector::new(0.0, 30000.0 * 0.8, 0.0), true);
                } else {
                    velocity.y = body.linvel().y;
                }
            }
            body.set_linvel(velocity.into(), true);
        }
    }
}

fn player_look(
    settings: Res<MovementSettings>,
    windows: Res<Windows>,
    mut state: ResMut<InputState>,
    motion: Res<Events<MouseMotion>>,
    mut bodies: ResMut<RigidBodySet>,
    mut query: QuerySet<(
        Query<&mut Transform, With<CameraTag>>,
        Query<&RigidBodyHandleComponent, With<BodyTag>>,
    )>,
) {
    let window = windows.get_primary().unwrap();
    for ev in state.reader_motion.iter(&motion) {
        if let Ok(mut transform) = query.q0_mut().single_mut() {
            if window.cursor_locked() {
                state.pitch -= (settings.sensitivity * SENSITIVITY_COEFF * ev.delta.y).to_radians();
            }
            // Clamp pitch to prevent looking straight up or down
            state.pitch = state.pitch.clamp(
                -std::f32::consts::PI / 2.0 + 2.0 * std::f32::consts::PI / 180.0,
                std::f32::consts::PI / 2.0 - 2.0 * std::f32::consts::PI / 180.0,
            );
            transform.rotation = Quat::from_axis_angle(Vec3::X, state.pitch);
        }
        if let Ok(body_handle) = query.q1().single() {
            if window.cursor_locked() {
                let body = bodies
                    .get_mut(body_handle.handle())
                    .expect("Failed to get player's ridigbody");
                state.yaw -= ev.delta.x * settings.sensitivity * SENSITIVITY_COEFF;
                let rot: UnitQuaternion<f32> = UnitQuaternion::new(
                    Vector::y()
                        * -(ev.delta.x * settings.sensitivity * SENSITIVITY_COEFF).to_radians(),
                );
                let mut next_pos = *body.position();
                next_pos.append_rotation_wrt_center_mut(&rot);
                body.set_position(next_pos, true);
            }
        }
    }
}

fn cursor_grab(input: Res<Kurinji>, mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    if input.is_action_active("PAUSE") {
        toggle_grab_cursor(window);
    }
}

fn mapping(mut kurinji: ResMut<Kurinji>, config: Res<CobbleConfig>) {
    kurinji.set_bindings(config.input.bindings.clone());
}

fn process_input(
    selection: Res<RaycastSelection>,
    mut input: EventReader<OnActionBegin>,
    mut mod_event: EventWriter<EventChunkAction>,
    mut inventory: ResMut<Inventory>,
    mut settings: ResMut<MovementSettings>,
    config: Res<CobbleConfig>,
) {
    for event in input.iter() {
        match event.action.as_str() {
            "SLOT_1" => inventory.switch_slot(0),
            "SLOT_2" => inventory.switch_slot(1),
            "SLOT_3" => inventory.switch_slot(2),
            "SLOT_4" => inventory.switch_slot(3),
            "SLOT_5" => inventory.switch_slot(4),
            "SLOT_6" => inventory.switch_slot(5),
            "SLOT_7" => inventory.switch_slot(6),
            "SLOT_8" => inventory.switch_slot(7),
            "SLOT_9" => inventory.switch_slot(8),
            "SLOT_10" => inventory.switch_slot(9),
            "FLY_TOGGLE" if config.game.creative => {
                settings.fly = !settings.fly;
            }
            "PICK_BLOCK" => {
                if let Some((chunk, index)) = selection.looking_at {
                    mod_event.send(EventChunkAction::PickBlock(chunk, index));
                }
            }
            "PLACE" => {
                if let (Some((chunk, index)), Some(norm)) = (selection.looking_at, selection.normal)
                {
                    if let Some(block_type) = inventory.consume_current_slot() {
                        let (norm_chunk, norm_index) = absolut_to_index_i32::<
                            { defaults::CHUNK_WIDTH },
                        >(
                            &(index_to_absolut::<{ defaults::CHUNK_WIDTH }>(chunk, index) + norm),
                        );
                        mod_event.send(EventChunkAction::ModifyBlock(
                            norm_chunk, norm_index, block_type, true,
                        ));
                    }
                }
            }
            "BREAK" => {
                if let Some((chunk, index)) = selection.looking_at {
                    mod_event.send(EventChunkAction::ModifyBlock(
                        chunk,
                        index,
                        BlockType::Air,
                        true,
                    ));
                }
            }
            _ => (),
        }
    }
}
