#[cfg(feature = "inline_assets")]
use std::{collections::HashMap, path::Path};

use bevy::{
    asset::HandleId,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

use crate::{config::CobbleConfig, inventory::Inventory, world::BlockType, AppState};

#[derive(Clone, PartialEq, Eq, Hash, Debug, SystemLabel)]
pub enum OverlayLabels {
    LoadAssets,
}

pub struct OverlayPlugin;

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(
            SystemSet::on_exit(AppState::Loading)
                .with_system(setup_overlay.system())
                .after(OverlayLabels::LoadAssets),
        )
        .add_system(update_fps_counter.system())
        .add_system(update_crosshair.system())
        .add_system(update_toolbar.system())
        .add_system_set(
            SystemSet::on_enter(AppState::Loading)
                .with_system(load_assets.system())
                .label(OverlayLabels::LoadAssets),
        )
        .insert_resource(Handles::default());
    }
}

const CROSSHAIR_SCALE: f32 = 0.0125;
const N_SLOTS: usize = 9;

#[derive(Default, Clone)]
pub struct Handles {
    crosshair: Handle<ColorMaterial>,
    inactive: Handle<ColorMaterial>,
    active: Handle<ColorMaterial>,

    font_mono: Handle<Font>,
    font_bold: Handle<Font>,

    dirt: Handle<ColorMaterial>,
    cobble: Handle<ColorMaterial>,
    grass: Handle<ColorMaterial>,
    planks: Handle<ColorMaterial>,
    sand: Handle<ColorMaterial>,
    gravel: Handle<ColorMaterial>,
    bricks: Handle<ColorMaterial>,
    wood: Handle<ColorMaterial>,
    leaves: Handle<ColorMaterial>,
}

#[cfg(not(feature = "inline_assets"))]
fn load_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(Handles::load(&asset_server, &mut *materials));
}

#[cfg(feature = "inline_assets")]
fn load_assets(
    mut commands: Commands,
    inline_asset_handles: Res<HashMap<&'static Path, HandleUntyped>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(Handles::load(&inline_asset_handles, &mut *materials));
}

impl Handles {
    #[cfg(not(feature = "inline_assets"))]
    fn load(asset_server: &Res<AssetServer>, materials: &mut Assets<ColorMaterial>) -> Self {
        macro_rules! load_texture_material {
            ($path:expr) => {
                materials.add(asset_server.load($path).into())
            };
        }

        Self {
            font_mono: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_bold: asset_server.load("fonts/FiraMono-Medium.ttf"),
            dirt: load_texture_material!("thumbs/dirt.png"),
            cobble: load_texture_material!("thumbs/cobble.png"),
            grass: load_texture_material!("thumbs/grass.png"),
            planks: load_texture_material!("thumbs/planks.png"),
            sand: load_texture_material!("thumbs/sand.png"),
            gravel: load_texture_material!("thumbs/gravel.png"),
            bricks: load_texture_material!("thumbs/bricks.png"),
            wood: load_texture_material!("thumbs/wood.png"),
            leaves: load_texture_material!("thumbs/leaves.png"),
            crosshair: load_texture_material!("images/crosshair.png"),
            inactive: load_texture_material!("images/toolbar_slot.png"),
            active: load_texture_material!("images/toolbar_slot_active.png"),
        }
    }

    #[cfg(feature = "inline_assets")]
    fn load(
        inline_asset_handles: &HashMap<&'static Path, HandleUntyped>,
        materials: &mut Assets<ColorMaterial>,
    ) -> Self {
        macro_rules! load_texture_material {
            ($path:expr) => {
                materials.add(
                    inline_asset_handles
                        .get(Path::new($path))
                        .unwrap()
                        .clone()
                        .typed()
                        .into(),
                )
            };
        }

        Self {
            font_mono: inline_asset_handles
                .get(Path::new("assets/fonts/FiraSans-Bold.ttf"))
                .unwrap()
                .clone()
                .typed(),
            font_bold: inline_asset_handles
                .get(Path::new("assets/fonts/FiraMono-Medium.ttf"))
                .unwrap()
                .clone()
                .typed(),
            dirt: load_texture_material!("assets/thumbs/dirt.png"),
            cobble: load_texture_material!("assets/thumbs/cobble.png"),
            grass: load_texture_material!("assets/thumbs/grass.png"),
            planks: load_texture_material!("assets/thumbs/planks.png"),
            sand: load_texture_material!("assets/thumbs/sand.png"),
            gravel: load_texture_material!("assets/thumbs/gravel.png"),
            bricks: load_texture_material!("assets/thumbs/bricks.png"),
            wood: load_texture_material!("assets/thumbs/wood.png"),
            leaves: load_texture_material!("assets/thumbs/leaves.png"),
            crosshair: load_texture_material!("assets/images/crosshair.png"),
            inactive: load_texture_material!("assets/images/toolbar_slot.png"),
            active: load_texture_material!("assets/images/toolbar_slot_active.png"),
        }
    }
}

/// Iterator of critial assets that need to be loaded before InGame is entered.
/// Adding transformative assets, such as `ColorMaterial`, might break the loading sequence.
impl IntoIterator for Handles {
    type Item = HandleId;
    type IntoIter = std::array::IntoIter<HandleId, 2>;

    fn into_iter(self) -> Self::IntoIter {
        std::array::IntoIter::new([self.font_mono.id, self.font_bold.id])
    }
}

fn setup_overlay(
    mut commands: Commands,
    config: Res<CobbleConfig>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    handles: ResMut<Handles>,
) {
    commands.spawn_bundle(UiCameraBundle::default());
    if config.debug.show_fps {
        debug!("Enabling fps overlay");
        commands
            .spawn_bundle(TextBundle {
                style: Style {
                    display: Display::None,
                    align_self: AlignSelf::FlexEnd,
                    margin: Rect {
                        left: Val::Percent(1.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                text: Text {
                    sections: vec![
                        TextSection {
                            value: "FPS: ".to_string(),
                            style: TextStyle {
                                font: handles.font_bold.clone(),
                                font_size: 26.0,
                                color: Color::WHITE,
                            },
                        },
                        TextSection {
                            value: "".to_string(),
                            style: TextStyle {
                                font: handles.font_mono.clone(),
                                font_size: 26.0,
                                color: Color::GOLD,
                            },
                        },
                    ],
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(FpsText);
    }
    commands
        .spawn_bundle(ImageBundle {
            style: Style {
                position_type: PositionType::Absolute,
                aspect_ratio: Some(1.0),
                ..Default::default()
            },
            material: handles.crosshair.clone(),
            ..Default::default()
        })
        .insert(Crosshair);
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                max_size: Size::new(Val::Percent(80.0), Val::Percent(10.0)),
                justify_content: JustifyContent::Center,
                padding: Rect {
                    left: Val::Percent(30.0),
                    right: Val::Percent(30.0),
                    bottom: Val::Percent(4.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            for i in 0..N_SLOTS {
                parent
                    .spawn_bundle(ImageBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            position: Rect {
                                left: Val::Px((90.0) * i as f32),
                                top: Val::Px(20.0),
                                ..Default::default()
                            },
                            margin: Rect::all(Val::Px(5.0)),
                            size: Size::new(Val::Px(90.0), Val::Px(90.0)),
                            align_items: AlignItems::Center,
                            align_content: AlignContent::Center,
                            justify_content: JustifyContent::Center,
                            ..Default::default()
                        },
                        material: handles.inactive.clone(),
                        ..Default::default()
                    })
                    .insert(ToolbarSlot(i))
                    .with_children(|parent| {
                        parent.spawn_bundle(ImageBundle {
                            style: Style {
                                position_type: PositionType::Relative,
                                margin: Rect::all(Val::Px(5.0)),
                                size: Size::new(Val::Percent(80.0), Val::Percent(80.0)),
                                ..Default::default()
                            },
                            material: handles.cobble.clone(),
                            ..Default::default()
                        });
                    });
            }
        });
}

struct ToolbarSlot(usize);
struct Crosshair;
struct FpsText;

fn update_fps_counter(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text, With<FpsText>>) {
    if let Ok(mut text) = query.single_mut() {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                text.sections[1].value = format!("{:.2}", average);
            }
        }
    }
}

fn update_crosshair(windows: Res<Windows>, mut query: Query<&mut Style, With<Crosshair>>) {
    if let Ok(mut style) = query.single_mut() {
        let window = windows.get_primary().unwrap();
        let scale = window.width().max(window.height()) as f32 * CROSSHAIR_SCALE;
        let (center_x, center_y) = (window.width() as f32 / 2.0, window.height() as f32 / 2.0);
        let position = (center_x - scale / 2.0, center_y - scale / 2.0);
        style.position.left = Val::Px(position.0);
        style.position.bottom = Val::Px(position.1);
        style.size.width = Val::Px(scale);
        style.size.height = Val::Px(scale);
    }
}

fn update_toolbar(
    inventory: Res<Inventory>,
    handles: ResMut<Handles>,
    windows: Res<Windows>,
    mut slot_query: Query<(
        &mut Handle<ColorMaterial>,
        &mut Style,
        &ToolbarSlot,
        &Children,
    )>,
    mut item_query: Query<(&mut Handle<ColorMaterial>, &mut Visible), Without<ToolbarSlot>>,
) {
    let window = windows.get_primary().unwrap();

    let slot_width: f32 = 50.0;
    let offset_from_bottom = window.height() * 0.025;
    let offset_from_left = (window.width() - slot_width * N_SLOTS as f32) / 2.0;
    for (mut material, mut style, ToolbarSlot(id), children) in slot_query.iter_mut() {
        style.position.left = Val::Px(offset_from_left + *id as f32 * slot_width);
        style.position.bottom = Val::Px(offset_from_bottom);
        style.size.width = Val::Px(slot_width);
        style.size.height = Val::Px(slot_width);
        *material = if *id == inventory.current_slot() {
            handles.active.clone()
        } else {
            handles.inactive.clone()
        };

        if let Some(child) = children.first() {
            if let Ok((mut block_, mut visible)) = item_query.get_mut(*child) {
                match inventory.item(*id) {
                    Some(block) => {
                        *block_ = match block {
                            BlockType::Dirt => handles.dirt.clone(),
                            BlockType::Cobble => handles.cobble.clone(),
                            BlockType::Grass => handles.grass.clone(),
                            BlockType::Planks => handles.planks.clone(),
                            BlockType::Sand => handles.sand.clone(),
                            BlockType::Bricks => handles.bricks.clone(),
                            BlockType::Leaves => handles.leaves.clone(),
                            BlockType::Wood => handles.wood.clone(),
                            BlockType::Gravel => handles.gravel.clone(),
                            _ => {
                                error!("No thumb for {:?}", block);
                                Handle::default()
                            }
                        };
                        visible.is_visible = true;
                    }
                    None => visible.is_visible = false,
                }
            }
        }
    }
}
