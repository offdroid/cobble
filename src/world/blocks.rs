use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Clone, Hash, Eq, Copy)]
#[repr(u8)]
pub enum BlockType {
    Air = 0,
    Dirt = 1,
    Grass = 2,
    Cobble = 3,
    Bricks = 4,
    Wood = 5,
    Planks = 6,
    Leaves = 7,
    Sand = 8,
    Gravel = 9,
}

pub const TEXTURE_LAYERS: u32 = 12;

pub const EXCEPT_AIR: [BlockType; 9] = [
    BlockType::Dirt,
    BlockType::Grass,
    BlockType::Cobble,
    BlockType::Bricks,
    BlockType::Wood,
    BlockType::Planks,
    BlockType::Leaves,
    BlockType::Sand,
    BlockType::Gravel,
];

pub const EXCEPT_NONE_MESH_GROUP: [MeshGroup; 1] = [MeshGroup::Cube];

lazy_static! {
    pub static ref EXCEPT_AIR_SET: HashSet<BlockType> =
        EXCEPT_AIR.iter().cloned().collect::<HashSet<_>>();

    pub static ref EXCEPT_NONE_MESH_GROUP_SET: HashSet<MeshGroup> = EXCEPT_NONE_MESH_GROUP
        .iter()
        .cloned()
        .collect::<HashSet<_>>();

    /// Map of how faces to texture ids for each rendered block.
    /// An entry is an array of [TOP, BOTTOM, LEFT, RIGHT, FRONT, BACK]
    pub static ref BLOCK_TEX_ID: HashMap<BlockType, [u32; 6]> = {
        use BlockType::*;

        let mut m = HashMap::with_capacity(9);
        m.insert(Dirt, [1; 6]);
        m.insert(Grass, [2, 1, 3, 3, 3, 3]);
        m.insert(Cobble, [4; 6]);
        m.insert(Planks, [5; 6]);
        m.insert(Sand, [6; 6]);
        m.insert(Bricks, [7; 6]);
        m.insert(Gravel, [8; 6]);
        m.insert(Leaves, [9; 6]);
        m.insert(Wood, [10, 10, 11, 11, 11, 11]);
        m
    };
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum MeshGroup {
    None,
    Cube,
}

pub struct BlockProperties {
    pub mesh_group: MeshGroup,
}

pub fn properties(block_type: &BlockType) -> BlockProperties {
    match block_type {
        BlockType::Dirt
        | BlockType::Grass
        | BlockType::Cobble
        | BlockType::Bricks
        | BlockType::Wood
        | BlockType::Planks
        | BlockType::Leaves
        | BlockType::Sand
        | BlockType::Gravel => BlockProperties {
            mesh_group: MeshGroup::Cube,
        },
        _ => BlockProperties {
            mesh_group: MeshGroup::None,
        },
    }
}
