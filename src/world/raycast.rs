use std::convert::TryInto;

use bevy::math::UVec3;
use bevy::{
    math::{swizzles::Vec3Swizzles, IVec2, IVec3, Vec3},
    prelude::*,
};

use crate::{
    interface::controller::CameraTag,
    world::{absolut_to_index, defaults, BlockType, NineSurroundChunk},
};

const MAX_REACH: f32 = 6.0;

/// System labels for ECS
#[derive(Clone, PartialEq, Eq, Hash, Debug, SystemLabel)]
pub enum RaycastLabels {
    Raycast,
}

pub struct VoxelRaycastPlugin;

impl Plugin for VoxelRaycastPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<RaycastSelection>().add_system_to_stage(
            CoreStage::PostUpdate,
            raycast_from_camera.system().label(RaycastLabels::Raycast), //),
        );
    }
}

/// Compute the intersection distance and normal of a ray with a cuboidal voxel if any.
///
/// Adapted from Majercik, A., Crassin, C., Shirley, P. and McGuire, M., 2018. _A ray-box intersection algorithm and efficient dynamic voxel rendering_. Journal of Computer Graphics Techniques Vol, 7(3).
/// Available online http://jcgt.org/published/0007/03/04/
///
/// # Arguments
/// * `position` - Absolute coordinates of the cube center. Note that this differs from the absolute index of a voxel by 0.5 on each axsis
/// * `ray_origin` - Origin of the ray, e.g., the camera position
/// * `ray_dir` - Direction of the the ray, e.g., based on rotation of the camera
///
/// # Alternative rapier/parry implementation
/// Seems to be a bit slower, however
/// ```
/// bevy_rapier3d::rapier::parry::query::RayCast::cast_ray_and_get_normal(
///     &bevy_rapier3d::rapier::geometry::Cuboid::new(nalgebra::Matrix3x1::from_element(BOX_RADIUS)),
///     &nalgebra::Isometry3::from_parts(
///         nalgebra::Translation3::new(box_center.x, box_center.y, box_center.z),
///         nalgebra::UnitQuaternion::identity(),
///     ),
///     &bevy_rapier3d::rapier::geometry::Ray::new(ray_origin.into(), ray_dir.into()),
///     6.0,
///     true,
/// )
/// .map(|v| (v.toi, v.normal.into()))
/// ```
pub fn intersect_box(box_center: Vec3, ray_origin: Vec3, ray_dir: Vec3) -> Option<(f32, Vec3)> {
    const ORIENTED: bool = false;
    const CAN_START_IN_BOX: bool = false;

    const BOX_RADIUS: f32 = 0.5;
    const INV_BOX_RADIUS: f32 = 1.0 / BOX_RADIUS;
    const BOX_ROT: Vec3 = Vec3::ZERO;
    let inv_ray_dir = 1.0 / ray_dir;

    let mut ray_origin = ray_origin - box_center;
    let mut ray_dir = ray_dir;
    if ORIENTED {
        ray_dir *= BOX_ROT;
        ray_origin *= BOX_ROT;
    }

    let winding: f32 =
        if CAN_START_IN_BOX && (ray_origin.abs() * INV_BOX_RADIUS).max_element() < 1.0 {
            -1.0
        } else {
            1.0
        };
    let mut sgn: Vec3 = -ray_dir.signum();
    // Distance to plane
    let mut d: Vec3 = BOX_RADIUS * winding * sgn - ray_origin;
    if ORIENTED {
        d /= ray_dir;
    } else {
        d *= inv_ray_dir;
    };

    fn test_component(u: f32, vm: fn(Vec3) -> Vec2, ray_origin: &Vec3, ray_dir: &Vec3) -> bool {
        u >= 0.0
            && (vm(*ray_origin) + vm(*ray_dir) * u)
                .abs()
                .cmplt(Vec2::from([BOX_RADIUS, BOX_RADIUS]))
                .all()
    }
    struct Bvec3 {
        x: bool,
        y: bool,
        z: bool,
    }
    let test = Bvec3 {
        x: test_component(d.x, Vec3Swizzles::yz, &ray_origin, &ray_dir),
        y: test_component(d.y, Vec3Swizzles::zx, &ray_origin, &ray_dir),
        z: test_component(d.z, Vec3Swizzles::xy, &ray_origin, &ray_dir),
    };
    sgn = if test.x {
        Vec3::new(sgn.x, 0.0, 0.0)
    } else if test.y {
        Vec3::new(0.0, sgn.y, 0.0)
    } else {
        Vec3::new(0.0, 0.0, if test.z { sgn.z } else { 0.0 })
    };

    let distance: f32 = if sgn.x != 0.0 {
        d.x
    } else if sgn.y != 0.0 {
        d.y
    } else {
        d.z
    };
    let normal = if ORIENTED { BOX_ROT * sgn } else { sgn };
    if (sgn.x != 0.0) || (sgn.y != 0.0) || (sgn.z != 0.0) {
        Some((distance, normal))
    } else {
        None
    }
}

/// Find the closest non-air voxel to the provided ray origin.
/// Returning the voxel chunk, index and normal vector if any assuming the relevant chunks are
/// in memory.
pub fn raycast_voxel(
    ray_origin: Vec3,
    ray_direction: Vec3,
    chunk_store: &NineSurroundChunk,
) -> Option<(IVec2, UVec3, IVec3)> {
    let mut min_distance = f32::INFINITY;
    let mut arg_min = None;

    // TODO Make more efficient by pruning
    // Find closest intersecting voxel using arg_min on the minimal intersection distance
    for x in -5..5 {
        for y in -5..5 {
            for z in -5..5 {
                // Calculate the center of the next voxel to check
                let v = ray_origin.as_i32().as_f32()
                    + IVec3::new(x, y, z).as_f32()
                    + Vec3::from([0.5; 3]);

                let (v_chunk, v_index) = absolut_to_index::<{ defaults::CHUNK_WIDTH }>(&v);
                if v_index.y >= defaults::CHUNK_HEIGHT.try_into().unwrap()
                    || v_index.x >= defaults::CHUNK_WIDTH.try_into().unwrap()
                    || v_index.z >= defaults::CHUNK_WIDTH.try_into().unwrap()
                {
                    continue;
                }
                if let Some((distance, normal)) = intersect_box(v, ray_origin, ray_direction) {
                    match chunk_store.data.get(&v_chunk) {
                        Some(chunk)
                            if distance < min_distance
                                && distance >= 0.0
                                && distance <= MAX_REACH =>
                        {
                            match chunk.voxel[v_index] {
                                BlockType::Air => continue,
                                _ => {
                                    min_distance = distance;
                                    arg_min = Some((v_chunk, v_index, normal.as_i32()));
                                }
                            }
                        }
                        None => {
                            debug!(
                                "Attempted to raycast with unloaded chunk. {} is missing",
                                v_chunk
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    arg_min
}

/// Block the player is looking at and normal (unit) vector indicating the looked at face
#[derive(Default)]
pub struct RaycastSelection {
    pub looking_at: Option<(IVec2, UVec3)>,
    pub normal: Option<IVec3>,
}

fn raycast_from_camera(
    mut selection: ResMut<RaycastSelection>,
    chunks: Res<NineSurroundChunk>,
    query: Query<&GlobalTransform, With<CameraTag>>,
) {
    if let Ok(global_transform) = query.single() {
        // Taken from bevy_mod_raycast, see README for license information
        // https://github.com/aevyrie/bevy_mod_raycast/blob/52a132745c04c9ac444f82dbc9acb1ad7b311e51/src/primitives.rs#L88
        let transform = global_transform.compute_matrix();
        let pick_position_ndc = Vec3::new(0.0, 0.0, -1.0);
        let pick_position = transform.project_point3(pick_position_ndc);
        let (_, _, source_origin) = transform.to_scale_rotation_translation();
        let ray_direction = pick_position - source_origin;

        // Compute the looked at voxel and the respective normal vector
        if let Some((sel_chunk, sel_index, sel_normal)) =
            raycast_voxel(source_origin, ray_direction, &chunks)
        {
            selection.looking_at = Some((sel_chunk, sel_index));
            selection.normal = if (sel_index.y == 0 && sel_normal.y < 0)
                || (sel_index.y >= defaults::CHUNK_HEIGHT as u32 - 1 && sel_normal.y > 0)
            {
                // Ignore normals that are above or below the chunk boundaries
                None
            } else {
                Some(sel_normal)
            };
        } else {
            selection.looking_at = None;
            selection.normal = None;
        }
    }
}
