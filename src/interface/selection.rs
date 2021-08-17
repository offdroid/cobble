use bevy::{
    prelude::*,
    render::{mesh, pipeline::PrimitiveTopology},
};

use crate::config::CobbleConfig;
use crate::world::{defaults, index_to_absolut, raycast::RaycastSelection};
use crate::AppState;

pub struct SelectionTag;
pub struct NormalSelectionTag;

pub struct SelectionHintPlugin;

impl Plugin for SelectionHintPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(
            SystemSet::on_enter(AppState::InGame).with_system(setup_selection_hint.system()),
        )
        .add_system_set(
            SystemSet::on_update(AppState::InGame).with_system(update_selection_hint.system()),
        );
    }
}

pub fn update_selection_hint(
    selection: Res<RaycastSelection>,
    mut query_selection: Query<
        (&mut Visible, &mut Transform),
        (With<SelectionTag>, Without<NormalSelectionTag>),
    >,
    mut query_normal_selection: Query<
        (&mut Visible, &mut Transform),
        (With<NormalSelectionTag>, Without<SelectionTag>),
    >,
) {
    if let Ok((mut draw, mut transform)) = query_selection.single_mut() {
        if let Some((chunk, index)) = selection.looking_at {
            transform.translation =
                index_to_absolut::<{ defaults::CHUNK_WIDTH }>(chunk, index).as_f32();
            draw.is_visible = true;
        } else {
            draw.is_visible = false;
        }
    }
    if let Ok((mut draw, mut transform)) = query_normal_selection.single_mut() {
        match (selection.looking_at, selection.normal) {
            (Some((chunk, index)), Some(norm)) => {
                transform.translation = index_to_absolut::<{ defaults::CHUNK_WIDTH }>(chunk, index)
                    .as_f32()
                    + norm.as_f32();
                draw.is_visible = true;
            }
            (_, _) => {
                draw.is_visible = false;
            }
        }
    }
}

pub fn setup_selection_hint(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<CobbleConfig>,
) {
    const VERTICES: [([f32; 3], [f32; 3], [f32; 2]); 8] = [
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([0.0, 1.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([1.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([1.0, 1.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([1.0, 1.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
    ];
    let indices = mesh::Indices::U32(vec![
        0, 1, 0, 2, 0, 3, 7, 4, 7, 5, 7, 6, 1, 5, 1, 4, 2, 4, 2, 6, 3, 5, 3, 6,
    ]);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for (position, normal, uv) in VERTICES.iter() {
        positions.push(*position);
        normals.push(*normal);
        uvs.push(*uv);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::LineList);
    mesh.set_indices(Some(indices));
    mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_xyz(0.0, 1.0, 0.0),
            mesh: meshes.add(mesh.clone()),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb_linear(1.0, 1.0, 1.0),
                double_sided: false,
                unlit: true,
                ..Default::default()
            }),
            ..Default::default()
        })
        .insert(SelectionTag);
    if config.debug.show_selection_normal {
        commands
            .spawn_bundle(PbrBundle {
                transform: Transform::from_xyz(0.0, 1.0, 0.0),
                mesh: meshes.add(mesh),
                material: materials.add(StandardMaterial {
                    base_color: Color::rgb_linear(1.0, 0.0, 0.0),
                    double_sided: false,
                    unlit: true,
                    ..Default::default()
                }),
                ..Default::default()
            })
            .insert(NormalSelectionTag);
    }
}
