use std::ops::RangeInclusive;

use bevy::{ecs::schedule::ShouldRun, prelude::*};
use bevy_rapier3d::{
    na::Isometry3,
    na::Translation3,
    na::UnitQuaternion,
    physics::{EventQueue, RigidBodyHandleComponent},
    rapier::{
        dynamics::{RigidBodyBuilder, RigidBodySet},
        geometry::{ColliderBuilder, ColliderSet, InteractionGroups},
    },
};

use crate::{config::CobbleConfig, interface::controller::MovementState};

use super::{defaults, index_to_absolut, BlockType, NineSurroundChunk, PlayerPosition};

pub const COLLIDER_PLAYER_UD: u128 = 1;
pub const COLLIDER_ENV_FLOOR_UD: u128 = 2;
pub const COLLIDER_ENV_OTHER_UD: u128 = 3;
pub const COLLIDER_PLAYER_SENSOR_UD: u128 = 4;

pub const COLLIDER_FLOOR_0_ID: u16 = 9 * 3;
pub const COLLIDER_SENSOR_ID: u16 = 9 * 3 + 1;
pub const COLLIDER_ALL_IDS: RangeInclusive<u16> = 0..=(3 * 9);

pub const GROUP_PLAYER: InteractionGroups = InteractionGroups::new(0b0000101, 0b000001);
pub const GROUP_PLAYER_SENSOR: InteractionGroups = InteractionGroups::new(0b0000110, 0b000010);
pub const GROUP_FLOOR: InteractionGroups = InteractionGroups::new(0b0000111, 0b000111);
pub const GROUP_ENV: InteractionGroups = InteractionGroups::new(0b0000101, 0b000111);

#[derive(Default)]
pub struct VoxelColliderState(IVec2, UVec3);
pub struct ColliderBlock(u16);

pub fn setup_collider(
    mut commands: Commands,
    config: Res<CobbleConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for i in COLLIDER_ALL_IDS {
        let is_floor_0 = i == COLLIDER_FLOOR_0_ID;

        let (user_data, collision_group, friction) = if is_floor_0 {
            (COLLIDER_ENV_FLOOR_UD, GROUP_FLOOR, 9.0)
        } else {
            (COLLIDER_ENV_OTHER_UD, GROUP_ENV, 0.0)
        };
        let collider = ColliderBuilder::cuboid(0.5, 0.5, 0.5)
            .friction(friction)
            .collision_groups(collision_group)
            .user_data(user_data);
        let rigid_body = RigidBodyBuilder::new_static()
            .user_data(user_data)
            .ccd_enabled(true);

        let mut collider_entity_cmds =
            commands.spawn_bundle((collider, rigid_body, ColliderBlock(i)));
        if config.debug.show_colliders {
            collider_entity_cmds.insert_bundle(PbrBundle {
                mesh: meshes.add(shape::Cube { size: 1.0 }.into()),
                material: materials.add(
                    (if is_floor_0 {
                        Color::rgb(0.9, 0.0, 0.1)
                    } else {
                        Color::rgb(0.0, 0.9, 0.1)
                    })
                    .into(),
                ),
                ..Default::default()
            });
        }
    }

    let airborn_collider = ColliderBuilder::capsule_z(0.2, 0.1)
        .sensor(true)
        .collision_groups(GROUP_PLAYER_SENSOR)
        .user_data(COLLIDER_PLAYER_SENSOR_UD);
    let airborn_ridig_body = RigidBodyBuilder::new_dynamic()
        .lock_translations()
        .lock_rotations()
        .ccd_enabled(true)
        .user_data(COLLIDER_PLAYER_SENSOR_UD);
    let mut sensor_entity_cmds = commands.spawn_bundle((
        airborn_collider,
        airborn_ridig_body,
        ColliderBlock(COLLIDER_SENSOR_ID),
    ));
    if config.debug.show_colliders {
        sensor_entity_cmds.insert_bundle(PbrBundle {
            mesh: meshes.add(
                shape::Icosphere {
                    radius: 0.1,
                    subdivisions: 4,
                }
                .into(),
            ),
            material: materials.add(Color::rgb(0.1, 0.1, 0.9).into()),
            ..Default::default()
        });
    }
}

pub fn run_criteria_update_colliders(
    chunk_store: Res<NineSurroundChunk>,
    position: Res<PlayerPosition>,
) -> ShouldRun {
    if chunk_store.is_changed() || position.is_changed() {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}
pub fn update_colliders(
    chunk_store: Res<NineSurroundChunk>,
    position: Res<PlayerPosition>,
    query: Query<(&RigidBodyHandleComponent, &ColliderBlock)>,
    mut bodies: ResMut<RigidBodySet>,
) {
    let far_away: Isometry3<f32> = Isometry3::from_parts(
        Translation3::new(0.0, -100.0, 0.0),
        UnitQuaternion::identity(),
    );
    query.for_each(|(body_handle, ColliderBlock(id))| {
        let body = bodies.get_mut(body_handle.handle()).unwrap();

        if id == &COLLIDER_SENSOR_ID {
            body.set_position(
                Isometry3::from_parts(
                    Translation3::new(
                        position.absolut.x,
                        position.absolut.y - 1.5,
                        position.absolut.z,
                    ),
                    UnitQuaternion::identity(),
                ),
                true,
            );
        } else {
            let (x, y, z) =
                index_to_absolut::<{ defaults::CHUNK_WIDTH }>(position.chunk, position.index)
                    .into();
            let collider_pos = if id != &COLLIDER_FLOOR_0_ID {
                let vertical_offset = match id.div_euclid(9) {
                    0 => 1,
                    1 => 0,
                    2 => -1,
                    _ => panic!("Unexpected collider id"),
                };

                IVec3::from(match id.rem_euclid(9) {
                    0 => [0, vertical_offset, 0],
                    1 => [1, vertical_offset, 0],
                    2 => [0, vertical_offset, 1],
                    3 => [1, vertical_offset, 1],
                    4 => [-1, vertical_offset, 0],
                    5 => [0, vertical_offset, -1],
                    6 => [-1, vertical_offset, -1],
                    7 => [1, vertical_offset, -1],
                    8 => [-1, vertical_offset, 1],
                    _ => [0, vertical_offset, 0],
                }) + IVec3::new(x, y, z)
            } else {
                IVec3::from([x, y - 2, z])
            }
            .as_f32();
            body.set_position(
                match chunk_store.get(&collider_pos) {
                    None => far_away,
                    Some(block) => {
                        if block != BlockType::Air {
                            Isometry3::from_parts(
                                Translation3::new(
                                    collider_pos.x + 0.5,
                                    collider_pos.y + 0.5,
                                    collider_pos.z + 0.5,
                                ),
                                UnitQuaternion::identity(),
                            )
                        } else {
                            far_away
                        }
                    }
                },
                true,
            );
        }
    });
}

pub fn compute_is_airborn(
    events: &EventQueue,
    collider_set: &ColliderSet,
    state: &mut MovementState,
) {
    while let Ok(intersection_event) = events.intersection_events.pop() {
        if let (Some(c1), Some(c2)) = (
            collider_set.get(intersection_event.collider1),
            collider_set.get(intersection_event.collider2),
        ) {
            if [c1.user_data, c2.user_data].contains(&COLLIDER_ENV_FLOOR_UD)
                && [c1.user_data, c2.user_data].contains(&COLLIDER_PLAYER_SENSOR_UD)
            {
                let lesser = if c1.user_data < c2.user_data {
                    intersection_event.collider1
                } else {
                    intersection_event.collider2
                }
                .into_raw_parts();
                if intersection_event.intersecting {
                    // Insert the floor id/user_data, it is always the smaller value of the two
                    state.intersections.insert(lesser);
                } else {
                    state.intersections.remove(&lesser);
                }
            } else {
                debug!("Other intersection {:?}", intersection_event)
            }
        };
    }

    state.airborn = state.intersections.is_empty();
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn collision_groups() {
        // Player sensor can only interact with the floor
        assert!(GROUP_PLAYER.test(GROUP_ENV));
        assert!(GROUP_ENV.test(GROUP_PLAYER));
        assert!(GROUP_PLAYER.test(GROUP_FLOOR));
        assert!(GROUP_FLOOR.test(GROUP_PLAYER));
        assert!(GROUP_FLOOR.test(GROUP_PLAYER_SENSOR));
        assert!(GROUP_PLAYER_SENSOR.test(GROUP_FLOOR));
        assert!(!GROUP_PLAYER_SENSOR.test(GROUP_PLAYER));
        assert!(!GROUP_PLAYER.test(GROUP_PLAYER_SENSOR));
        assert!(!GROUP_PLAYER_SENSOR.test(GROUP_ENV));
        assert!(!GROUP_ENV.test(GROUP_PLAYER_SENSOR));
    }
}
