pub mod blocks;
pub mod generator;
pub mod physics;
pub mod raycast;
pub mod voxel;

use std::collections::{HashMap, HashSet};

#[cfg(feature = "inline_assets")]
use std::path::Path;

use bevy::{
    asset::{HandleId, LoadState},
    ecs::schedule::ShouldRun,
    math::{IVec2, Vec3},
    pbr::AmbientLight,
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        texture::{AddressMode, SamplerDescriptor},
    },
    tasks::AsyncComputeTaskPool,
};
use bevy_rapier3d::physics::RapierConfiguration;

use crate::{
    config::CobbleConfig,
    interface::controller::{CameraTag, ControllerLabels},
    inventory::Inventory,
    shader, AppState,
};

pub(super) use self::blocks::*;
pub(super) use self::generator::*;
pub(super) use self::physics::*;
pub(super) use self::voxel::*;

#[derive(Clone, PartialEq, Eq, Hash, Debug, SystemLabel)]
enum WorldLabels {
    VoxelModification,
    Movement,
    ChunkLoad,
    ChunkMesh,
    UpdateColliders,
}

pub struct WorldPlugin;
impl Plugin for WorldPlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.insert_resource(NineSurroundChunk::empty())
            .insert_resource(Handles::default())
            .insert_resource(PlayerPosition::default())
            .add_event::<EventChunkCommand>()
            .add_event::<EventChunkAction>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                movement.system().label(WorldLabels::Movement),
            )
            .add_system_set(
                SystemSet::on_update(AppState::InGame)
                    .with_system(voxel_action.system())
                    .label(WorldLabels::VoxelModification),
            )
            .add_system_set(
                SystemSet::on_update(AppState::InGame)
                    .with_system(chunk_load.system())
                    .label(WorldLabels::ChunkLoad)
                    .after(WorldLabels::VoxelModification),
            )
            .add_system_set(
                SystemSet::on_update(AppState::InGame)
                    .with_system(chunk_mesh.system())
                    .with_run_criteria(run_criteria_chunk_mesh.system())
                    .label(WorldLabels::ChunkMesh)
                    .after(WorldLabels::ChunkLoad),
            )
            .add_system_set(
                SystemSet::on_update(AppState::InGame)
                    .with_system(update_colliders.system())
                    .with_run_criteria(run_criteria_update_colliders.system())
                    .label(WorldLabels::UpdateColliders)
                    .before(ControllerLabels::PlayerMove),
            )
            .add_system_set(
                SystemSet::on_enter(AppState::InGame).with_system(setup_collider.system()),
            )
            .add_system_set(
                SystemSet::on_enter(AppState::InGame).with_system(setup_lights.system()),
            )
            .add_system_set(
                SystemSet::on_update(AppState::InGame).with_system(update_lights.system()),
            )
            .add_system_set(
                SystemSet::on_exit(AppState::Loading).with_system(initial_chunk_load.system()),
            )
            .add_system_set(
                SystemSet::on_enter(AppState::Loading).with_system(load_textures.system()),
            )
            .add_system_set(
                SystemSet::on_update(AppState::Loading).with_system(create_atlas.system()),
            );
    }
}

#[cfg(not(feature = "inline_assets"))]
fn load_textures(
    mut handles: ResMut<Handles>,
    asset_server: Res<AssetServer>,
    mut rapier: ResMut<RapierConfiguration>,

    #[cfg(feature = "inline_assets")] inline_asset_handles: Res<
        HashMap<&'static Path, HandleUntyped>,
    >,
) {
    handles.atlas = asset_server.load("images/atlas.png");
    // Deactive the physics pipeline
    rapier.physics_pipeline_active = false;
    rapier.query_pipeline_active = false;
}

#[cfg(feature = "inline_assets")]
fn load_textures(
    mut handles: ResMut<Handles>,
    mut rapier: ResMut<RapierConfiguration>,
    inline_asset_handles: Res<HashMap<&'static Path, HandleUntyped>>,
) {
    handles.atlas = inline_asset_handles
        .get(Path::new("assets/images/atlas.png"))
        .unwrap()
        .clone()
        .typed();
    // Deactive the physics pipeline
    rapier.physics_pipeline_active = false;
    rapier.query_pipeline_active = false;
}

fn initial_chunk_load(
    chunk_store: ResMut<NineSurroundChunk>,
    mut event: EventWriter<EventChunkCommand>,
    mut rapier: ResMut<RapierConfiguration>,
) {
    // Load the surrounding chunks on startup
    for missing_chunk in chunk_store.missing_chunks(&Vec3::ZERO) {
        event.send(EventChunkCommand::Load(missing_chunk));
    }
    // Active the physics pipeline
    rapier.physics_pipeline_active = true;
    rapier.query_pipeline_active = true;
}

struct SunTag;

fn setup_lights(mut commands: Commands, mut ambient_light: ResMut<AmbientLight>) {
    commands
        .spawn_bundle(LightBundle {
            transform: Transform::from_xyz(5.0, 50.0, 5.0),
            light: Light {
                color: Color::rgb(1.0, 1.0, 1.0),
                intensity: 40000.0,
                range: 400.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(SunTag);

    ambient_light.color = Color::WHITE;
    ambient_light.brightness = 0.8;
}

/// Event to request loading or unloading of a specific chunk
pub enum EventChunkCommand {
    Update(IVec2),
    Load(IVec2),
    Unload(IVec2),
}

pub enum EventChunkAction {
    /// Event to replace a single block defined by its chunk, index, block_type, and whether the
    /// player should absorb/pickup the destroyed block, if applicable
    ModifyBlock(IVec2, UVec3, BlockType, bool),
    PickBlock(IVec2, UVec3),
}

#[derive(Clone)]
struct ChunkEntitySet(HashMap<IVec2, (HashSet<Entity>, HashSet<MeshGroup>)>);

/// Stores handles to the current loaded meshes and other related assets, such as materials
#[derive(Clone)]
pub struct Handles {
    chunks: HashMap<(IVec2, MeshGroup), Handle<Mesh>>,
    chunks_entities: ChunkEntitySet,
    atlas: Handle<Texture>,
    atlas_material: Handle<StandardMaterial>,
    pipeline: Handle<PipelineDescriptor>,
}

impl IntoIterator for Handles {
    type Item = HandleId;
    type IntoIter = std::array::IntoIter<HandleId, 1>;

    fn into_iter(self) -> Self::IntoIter {
        std::array::IntoIter::new([self.atlas.id])
    }
}

impl ChunkEntitySet {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn get_mut(&mut self, index: &IVec2) -> &mut (HashSet<Entity>, HashSet<MeshGroup>) {
        self.0
            .entry(*index)
            .or_insert_with(|| (HashSet::new(), HashSet::new()))
    }

    fn remove_by_block_type(&mut self, _index: &IVec2, _block_typee: BlockType) {}
}

impl Default for Handles {
    fn default() -> Self {
        Self {
            chunks: HashMap::with_capacity(9),
            chunks_entities: ChunkEntitySet::new(),
            atlas: Default::default(),
            atlas_material: Default::default(),
            pipeline: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct PlayerPosition {
    pub absolut: Vec3,
    /// Chunk the player is in
    pub chunk: IVec2,
    /// Position inside the chunk
    pub index: UVec3,
}

struct AssociatedChunk {
    chunk: IVec2,
    mesh_group: MeshGroup,
}

fn create_atlas(
    asset_server: Res<AssetServer>,
    mut textures: ResMut<Assets<Texture>>,
    mut handles: ResMut<Handles>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut loaded: Local<bool>,
) {
    if *loaded || asset_server.get_load_state(&handles.atlas) != LoadState::Loaded {
        return;
    }

    if let bevy::asset::LoadState::Loaded = asset_server.get_load_state(&handles.atlas) {
        let mut texture = textures.get_mut(&handles.atlas).unwrap();
        texture.sampler = SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            ..Default::default()
        };
        texture.reinterpret_stacked_2d_as_array(blocks::TEXTURE_LAYERS);
        handles.atlas_material = materials.add(StandardMaterial {
            base_color_texture: Some(handles.atlas.clone()),
            roughness: 0.5,
            metallic: 0.1,
            reflectance: 0.2,
            unlit: false,
            ..Default::default()
        });

        let pbr_pipeline = shader::build_pbr_pipeline(&mut shaders);
        let pipeline_handle = pipelines.add(pbr_pipeline);
        handles.pipeline = pipeline_handle;
        *loaded = true;
    }
}

fn voxel_action(
    mut chunk_store: ResMut<NineSurroundChunk>,
    mut chunk_mod: EventReader<EventChunkAction>,
    mut voxel_update: EventWriter<EventChunkCommand>,
    mut inventory: ResMut<Inventory>,
    config: Res<CobbleConfig>,
) {
    let mut voxels_to_update = HashSet::new();
    for event in chunk_mod.iter() {
        match *event {
            EventChunkAction::ModifyBlock(chunk, index, block_type, absorb) => {
                if !config.game.breakable_bedrock && index.y == 0 {
                    return;
                }
                if let Some(chunk_data) = chunk_store.data.get_mut(&chunk) {
                    if absorb && block_type == BlockType::Air {
                        inventory.absorb(chunk_data.voxel[index], 1);
                    }
                    chunk_data.voxel[index] = block_type;
                    voxels_to_update.insert(chunk);
                }
            }
            EventChunkAction::PickBlock(chunk, index) if config.game.creative => {
                if let Some(chunk_data) = chunk_store.data.get_mut(&chunk) {
                    inventory.absorb_creative(chunk_data.voxel[index]);
                }
            }
            _ => {}
        }
    }
    voxel_update.send_batch(
        voxels_to_update
            .iter()
            .map(|voxel| EventChunkCommand::Update(*voxel)),
    );
}

fn movement(
    chunk_store: Res<NineSurroundChunk>,
    query: Query<&GlobalTransform, With<CameraTag>>,
    mut position: ResMut<PlayerPosition>,
    mut last_chunk: Local<IVec2>,
    mut event_chunk: EventWriter<EventChunkCommand>,
) {
    if let Ok(transform) = query.single() {
        position.absolut = transform.translation;
        let (new_chunk, new_index) =
            absolut_to_index::<{ defaults::CHUNK_WIDTH }>(&transform.translation);
        if new_chunk != *last_chunk {
            *last_chunk = new_chunk;
            debug!("Entered new chunk ({}, {})", new_chunk.x, new_chunk.y);

            for missing_chunk in chunk_store.missing_chunks(&transform.translation) {
                event_chunk.send(EventChunkCommand::Load(missing_chunk));
            }
            // TODO Add some way of unloading old chunks
        }
        position.chunk = new_chunk;
        position.index = new_index;
    }
}

/// Update the position of the sun-light relative to the player position on the x- and z-axis
fn update_lights(mut query: Query<&mut Transform, With<SunTag>>, position: Res<PlayerPosition>) {
    if let Ok(mut transform) = query.single_mut() {
        let light_position = Vec3::from([
            position.absolut.x + 20.0,
            defaults::CHUNK_HEIGHT as f32 + 30.0,
            position.absolut.z + 20.0,
        ]);
        *transform = Transform::from_translation(light_position);
    }
}

/// Seed used for world generation
#[derive(Default, Copy, Clone)]
pub struct Seed(u32);

/// Generate or load a chunk (only the voxel data) into the chunk store on request. This also include unloading chunks
fn chunk_load(
    mut chunk_store: ResMut<NineSurroundChunk>,
    mut event_chunk: EventReader<EventChunkCommand>,
    seed: Option<Res<Seed>>,
    _commands: Commands,
    _thread_pool: Res<AsyncComputeTaskPool>,
) {
    let seed = seed.map_or_else(|| 0u32, |s| s.0);
    // Here we only generate new chunks from the world generator as opposed to loading them from
    // the disk
    for event in event_chunk.iter() {
        match event {
            EventChunkCommand::Load(index) => {
                if chunk_store.data.contains_key(index) {
                    return;
                }
                if chunk_store
                    .data
                    .insert(*index, BasicWorld::chunk(*index, seed))
                    .is_some()
                {
                    info!("Loaded (overrode) an already loaded chunk at {}", index);
                }
                chunk_store.reset_age(index);
            }
            EventChunkCommand::Unload(index) => {
                // Unload chunk data by removing its voxel data
                if chunk_store.data.remove(index).is_none() {
                    error!(
                        "Request to unload chunk at {} failed because it was not loaded",
                        index
                    );
                    return;
                }
            }
            EventChunkCommand::Update(_) => {
                // Chunk is already in memory, no further actions needed here
            }
        }
    }
    chunk_store.increment_age();
}

pub fn run_criteria_chunk_mesh(chunk_store: Res<NineSurroundChunk>) -> ShouldRun {
    if chunk_store.is_changed() {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

/// Build/update a chunk mesh for a load request and remove a mesh on a unload request
fn chunk_mesh(
    mut commands: Commands,
    chunk_store: ResMut<NineSurroundChunk>,
    mut handles: ResMut<Handles>,
    mut event_chunk: EventReader<EventChunkCommand>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for event in event_chunk.iter() {
        match event {
            EventChunkCommand::Load(index) | EventChunkCommand::Update(index) => {
                let new_meshes = match chunk_store.data.get(index) {
                    Some(chunk) => chunk.build(),
                    None => panic!(
                        "Chunk {} was requested to be meshed, but is not loaded. Loaded chunks are {:?}",
                        index,
                        chunk_store.data.keys()
                    ),
                };

                for (mesh_group, new_mesh) in new_meshes {
                    if let Some(new_mesh) = new_mesh {
                        let meta_index = (*index, mesh_group);
                        // If the mesh already exists then update its mesh, otherwise create a new entity
                        if let Some(handle) = handles.chunks.get(&meta_index).cloned() {
                            debug!("Reloading previously meshed chunk {:?}", meta_index);
                            //unimplemented!();
                            handles
                                .chunks
                                .insert(meta_index, meshes.set(handle, new_mesh));
                        } else {
                            let handle = meshes.add(new_mesh);
                            handles.chunks.insert(meta_index, handle.clone());
                            let _id = commands
                                .spawn_bundle(PbrBundle {
                                    mesh: handle,
                                    material: handles.atlas_material.clone(),
                                    render_pipelines: RenderPipelines::from_pipelines(vec![
                                        RenderPipeline::new(handles.pipeline.clone()),
                                    ]),
                                    visible: Visible {
                                        is_transparent: true,
                                        ..Default::default()
                                    },
                                    transform: Transform::from_xyz(
                                        (index.x * defaults::CHUNK_WIDTH as i32) as f32,
                                        0.0,
                                        (index.y * defaults::CHUNK_WIDTH as i32) as f32,
                                    ),
                                    ..Default::default()
                                })
                                .insert(AssociatedChunk {
                                    chunk: *index,
                                    mesh_group,
                                })
                                .id();
                        }
                    }
                }
            }
            EventChunkCommand::Unload(index) => {
                for mesh_group in blocks::EXCEPT_NONE_MESH_GROUP.iter() {
                    handles
                        .chunks
                        .get(&(*index, *mesh_group))
                        .map(|handle| meshes.remove(handle));
                }
                unimplemented!();
            }
        }
    }
}

pub struct NineSurroundChunk {
    pub data: HashMap<IVec2, GameChunk>,
    age: HashMap<IVec2, u8>,
}

impl NineSurroundChunk {
    pub fn get(&self, absolut: &Vec3) -> Option<BlockType> {
        let (chunk, index) = absolut_to_index::<{ defaults::CHUNK_WIDTH }>(absolut);
        assert!((index.x as usize) < defaults::CHUNK_WIDTH);
        assert!((index.z as usize) < defaults::CHUNK_WIDTH);
        if index.y >= defaults::CHUNK_HEIGHT as u32 {
            return None;
        }
        self.data.get(&chunk).map(|chunk| chunk.voxel[index])
    }

    fn from_data(data: HashMap<IVec2, GameChunk>) -> Self {
        Self {
            data,
            age: HashMap::new(),
        }
    }
}

impl ChunkManager for NineSurroundChunk {
    fn empty() -> Self {
        Self {
            data: HashMap::new(),
            age: HashMap::new(),
        }
    }

    fn reset_age(&mut self, index: &IVec2) {
        self.age.get_mut(index).map(|v| *v = 0);
    }

    fn increment_age(&mut self) {
        self.age.iter_mut().for_each(|(_, v)| {
            *v += 1;
        });
    }

    fn too_old(self, threshold: u8) -> Vec<IVec2> {
        let mut dealloc = Vec::new();
        for (k, v) in self.age {
            if v > threshold {
                dealloc.push(k);
            }
        }
        dealloc
    }

    fn neighborhood(&self, position: &Vec3) -> Vec<IVec2> {
        // Primitive neighborhood based on the surrounding chunks
        let in_chunk = InChunk::<{ defaults::CHUNK_WIDTH }>::in_chunk(position);
        let mut neighborhood = Vec::with_capacity(9);
        for i in 0..9i32 {
            neighborhood.push(IVec2::new(
                in_chunk.x - 1 + i.rem_euclid(3),
                in_chunk.y - 1 + i.div_euclid(3),
            ));
        }
        neighborhood
    }

    fn missing_chunks(&self, position: &Vec3) -> Vec<IVec2> {
        self.neighborhood(position)
            .into_iter()
            .filter(|chunk| !self.data.contains_key(chunk))
            .collect()
    }

    fn insert(&mut self, _: IVec2, _: GameChunk) -> bool {
        unimplemented!()
    }
    fn remove(&mut self, _: IVec2) -> bool {
        unimplemented!()
    }
}

pub trait ChunkManager {
    fn empty() -> Self;

    fn reset_age(&mut self, position: &IVec2);

    fn increment_age(&mut self);

    fn too_old(self, threshold: u8) -> Vec<IVec2>;

    /// Neighborhood of chunks given a position
    fn neighborhood(&self, position: &Vec3) -> Vec<IVec2>;

    /// Retrieve a list of currently not loaded chunks (aka missing) which are to be loaded
    fn missing_chunks(&self, position: &Vec3) -> Vec<IVec2>;

    fn insert(&mut self, index: IVec2, chunk: GameChunk) -> bool;
    fn remove(&mut self, index: IVec2) -> bool;
}

pub mod defaults {
    pub const CHUNK_SHAPE: [usize; 3] = [CHUNK_WIDTH, CHUNK_HEIGHT, CHUNK_WIDTH];

    pub const CHUNK_WIDTH: usize = 16;
    pub const CHUNK_HEIGHT: usize = 32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_surround_neighborhood() {
        let mut loaded_chunks: HashMap<IVec2, GameChunk> = HashMap::new();
        loaded_chunks.insert(
            IVec2::new(1, 0),
            GameChunk {
                voxel: VoxelChunk::air(defaults::CHUNK_SHAPE).into(),
                index: IVec2::new(1, 0),
            },
        );
        let c = NineSurroundChunk::from_data(loaded_chunks);
        let position = Vec3::new(0.0, 9.0, 0.0);

        let neighborhood = c.neighborhood(&position);
        const REF_NEIGBORHOOD: [(i32, i32); 9] = [
            (-1, 1),
            (0, 1),
            (1, 1),
            (-1, 0),
            (0, 0),
            (1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ];
        for ref_chunk in REF_NEIGBORHOOD.iter() {
            assert!(neighborhood.contains(&IVec2::new(ref_chunk.0, ref_chunk.1)));
        }
        let missing = c.missing_chunks(&position);
        assert!(!missing.contains(&IVec2::new(1, 0)));
        assert_eq!(missing.len(), neighborhood.len() - 1);
    }
}
