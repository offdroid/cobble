mod config;
mod interface;
mod inventory;
mod shader;
mod utils;
mod world;

use bevy::{asset::LoadState, prelude::*};

use bevy_rapier3d::{
    physics::{PhysicsInterpolationComponent, RapierConfiguration, RapierPhysicsPlugin},
    rapier::{dynamics::RigidBodyBuilder, geometry::ColliderBuilder},
};
use interface::controller::{BodyTag, CameraTag, NoCameraPlayerPlugin, YawTag};
#[cfg(not(feature = "inline_assets"))]
use interface::overlay;

use interface::overlay::OverlayPlugin;
use kurinji::KurinjiPlugin;
use world::raycast::VoxelRaycastPlugin;
use world::ChunkManager;

#[cfg(feature = "inline_assets")]
use crate::utils::inline_assets::{InlineAssets, InlineAssetsPlugin};
#[cfg(feature = "inline_assets")]
use bevy::asset::AssetPlugin;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

use config::CobbleConfig;

use std::collections::HashMap;
use std::path::Path;

use crate::interface::{overlay::OverlayLabels, selection::SelectionHintPlugin};
use crate::{inventory::Inventory, world::WorldPlugin};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    InGame,
    Loading,
}

#[bevy_main]
fn main() {
    let config: CobbleConfig = if cfg!(target_arch = "wasm32") {
        CobbleConfig::default()
    } else {
        config::load()
    };
    if config.debug.print_default_config {
        println!("{}", CobbleConfig::default_as_yaml().unwrap());
    }

    let mut app = App::build();
    app.insert_resource(Msaa {
        samples: config.video.msaa_samples,
    })
    .insert_resource(config.clone());

    #[cfg(feature = "inline_assets")]
    {
        let inline_assets = inline_assets![
            "assets/images/atlas.png",
            "assets/fonts/FiraSans-Bold.ttf",
            "assets/fonts/FiraMono-Medium.ttf",
            "assets/images/crosshair.png",
            "assets/images/toolbar_slot.png",
            "assets/images/toolbar_slot_active.png",
            "assets/thumbs/bricks.png",
            "assets/thumbs/cobble.png",
            "assets/thumbs/dirt.png",
            "assets/thumbs/grass.png",
            "assets/thumbs/gravel.png",
            "assets/thumbs/leaves.png",
            "assets/thumbs/planks.png",
            "assets/thumbs/sand.png",
            "assets/thumbs/wood.png",
        ];
        app.insert_resource(inline_assets);
    }
    #[cfg(not(target_arch = "wasm32"))]
    app.insert_resource(WindowDescriptor {
        title: "Cobble".to_string(),
        vsync: config.video.vsync,
        mode: config.video.to_window_mode(),
        ..Default::default()
    });

    app.add_plugins_with(DefaultPlugins, |group| {
        #[cfg(feature = "inline_assets")]
        group.add_after::<AssetPlugin, _>(InlineAssetsPlugin);

        group
    })
    .insert_resource(State::new(AppState::Loading))
    .add_state(AppState::Loading)
    .init_resource::<HashMap<&'static Path, HandleUntyped>>()
    .add_plugin(FrameTimeDiagnosticsPlugin::default())
    .add_plugin(RapierPhysicsPlugin)
    .insert_resource(RapierConfiguration {
        time_dependent_number_of_timesteps: true,
        // Disable physics until all assets are loaded
        physics_pipeline_active: false,
        query_pipeline_active: false,
        ..Default::default()
    })
    .add_plugin(KurinjiPlugin::default())
    .add_plugin(VoxelRaycastPlugin);
    if config.video.show_interface {
        app.add_plugin(OverlayPlugin);
    }

    #[cfg(feature = "inline_assets")]
    app.add_startup_system(setup_inline_assets.system());
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    app.add_plugin(NoCameraPlayerPlugin)
        .add_startup_system(setup_player.system());
    if config.debug.log_diagnostics {
        app.add_plugin(LogDiagnosticsPlugin::default());
    }
    app.insert_resource(world::NineSurroundChunk::empty())
        .add_plugin(WorldPlugin)
        .insert_resource(if config.game.creative {
            Inventory::creative_preset()
        } else {
            Inventory::survival_preset()
        })
        .add_system_set(
            SystemSet::on_update(AppState::Loading)
                .with_system(check_loading_finished.system())
                .after(OverlayLabels::LoadAssets),
        );

    if config.debug.show_selection {
        app.add_plugin(SelectionHintPlugin);
    }

    app.insert_resource(ClearColor(Color::rgb(0.82, 0.96, 0.96)));
    app.run();
}

#[cfg(feature = "inline_assets")]
fn setup_inline_assets(
    inline_assets: Res<InlineAssets>,
    asset_server: Res<AssetServer>,
    mut inline_asset_handles: ResMut<HashMap<&'static Path, HandleUntyped>>,
) {
    *inline_asset_handles = inline_assets.load_all(asset_server);
}

const SPAWN_POSITION: [f32; 3] = [0.0, 10.0, 0.0];

fn setup_player(mut commands: Commands) {
    let spawn_position = Vec3::from(SPAWN_POSITION);
    let body_rigid_body = RigidBodyBuilder::new_dynamic()
        .translation(spawn_position.x, spawn_position.y, spawn_position.z)
        .additional_mass(75.0)
        .linear_damping(1.0)
        .restrict_rotations(false, false, false)
        .user_data(world::COLLIDER_PLAYER_UD);
    let body_collider = ColliderBuilder::round_cylinder(0.8, 0.1, 0.0)
        .collision_groups(world::GROUP_PLAYER)
        .user_data(world::COLLIDER_PLAYER_UD);
    let body = commands
        .spawn_bundle((
            Transform::identity(),
            GlobalTransform::identity(),
            BodyTag,
            body_rigid_body,
            body_collider,
            PhysicsInterpolationComponent::new(spawn_position, Quat::IDENTITY),
        ))
        .id();
    let yaw = commands
        .spawn_bundle((GlobalTransform::identity(), Transform::identity(), YawTag))
        .id();
    let camera = commands
        .spawn_bundle(PerspectiveCameraBundle {
            global_transform: GlobalTransform::identity(),
            transform: Transform::from_matrix(Mat4::from_rotation_translation(
                Quat::from_axis_angle(Vec3::X, 0.0),
                Vec3::from([0.0, 0.3, 0.0]),
            )),
            perspective_projection: bevy::render::camera::PerspectiveProjection {
                fov: std::f32::consts::PI / 3.0,
                near: 0.01,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(CameraTag)
        .id();
    commands.entity(body).push_children(&[yaw]);
    commands.entity(yaw).push_children(&[camera]);
}

#[cfg(not(feature = "inline_assets"))]
fn check_loading_finished(
    asset_server: Res<AssetServer>,
    mut state: ResMut<State<AppState>>,
    mut loaded: Local<bool>,
    world_handles: Res<world::Handles>,
    overlay_handles: Res<overlay::Handles>,
) {
    if !*loaded
        && asset_server.get_group_load_state(
            world_handles
                .clone()
                .into_iter()
                .chain(overlay_handles.clone().into_iter()),
        ) == LoadState::Loaded
    {
        state.set(AppState::InGame).unwrap();
        *loaded = true;
    }
}

#[cfg(feature = "inline_assets")]
fn check_loading_finished(
    asset_server: Res<AssetServer>,
    inline_asset_handles: Res<HashMap<&'static Path, HandleUntyped>>,
    mut state: ResMut<State<AppState>>,
    mut loaded: Local<bool>,
) {
    if !*loaded
        && asset_server.get_group_load_state(inline_asset_handles.values().map(|h| h.id))
            == LoadState::Loaded
    {
        state.set(AppState::InGame).unwrap();
        *loaded = true;
    }
}
