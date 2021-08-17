use super::{blocks, defaults, BlockType};
use bevy::{prelude::*, render::pipeline::PrimitiveTopology};
use blocks::MeshGroup;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    ops::{Index, IndexMut},
};

pub type Block = super::blocks::BlockType;

#[derive(Serialize, Deserialize, Clone)]
pub struct VoxelChunk<T: Sized>(ndarray::Array3<T>, [usize; 3]);

impl<T: Sized + Clone> VoxelChunk<T> {
    #[allow(dead_code)]
    pub fn new(size: [usize; 3], fill: T) -> Self {
        Self(ndarray::Array3::from_elem(size, fill), size)
    }

    #[allow(dead_code)]
    pub fn shape(&self) -> [usize; 3] {
        self.1
    }

    pub fn width(&self) -> usize {
        self.1[0]
    }

    pub fn height(&self) -> usize {
        self.1[1]
    }

    pub fn depth(&self) -> usize {
        self.1[2]
    }

    pub fn indexed_iter(&self) -> ndarray::iter::IndexedIter<T, ndarray::Ix3> {
        self.0.indexed_iter()
    }
}

impl VoxelChunk<Block> {
    pub fn air(size: [usize; 3]) -> Self {
        Self(ndarray::Array3::from_elem(size, BlockType::Air), size)
    }
}

impl<T: Sized + Clone> VoxelChunk<T> {
    fn safe_get(&self, x: i32, y: i32, z: i32) -> Option<&T> {
        if 0 > x
            || x >= self.width() as i32
            || 0 > y
            || y >= self.height() as i32
            || 0 > z
            || z >= self.depth() as i32
        {
            None
        } else {
            Some(&self.0[(x as usize, y as usize, z as usize)])
        }
    }
}

impl<T: Sized + Clone> Index<UVec3> for VoxelChunk<T> {
    type Output = T;

    fn index(&self, index: UVec3) -> &Self::Output {
        let x = index.x as usize;
        let y = index.y as usize;
        let z = index.z as usize;
        if x >= self.width() || y >= self.height() || z >= self.width() {
            panic!("Out of index access");
        } else {
            &self.0[(x, y, z)]
        }
    }
}

impl<T: Sized + Clone> IndexMut<UVec3> for VoxelChunk<T> {
    fn index_mut(&mut self, index: UVec3) -> &mut Self::Output {
        let x = index.x as usize;
        let y = index.y as usize;
        let z = index.z as usize;
        if x >= self.width() || y >= self.height() || z >= self.width() {
            panic!("Out of index access");
        } else {
            &mut self.0[(x, y, z)]
        }
    }
}

impl<T: Sized + Clone> Index<(usize, usize, usize)> for VoxelChunk<T> {
    type Output = T;

    fn index(&self, index: (usize, usize, usize)) -> &Self::Output {
        if index.0 >= self.width() || index.1 >= self.height() || index.2 >= self.depth() {
            panic!("Out of index access");
        } else {
            &self.0[index]
        }
    }
}

impl<T: Sized + Clone> IndexMut<(usize, usize, usize)> for VoxelChunk<T> {
    fn index_mut(&mut self, index: (usize, usize, usize)) -> &mut Self::Output {
        if index.0 >= self.width() || index.1 >= self.height() || index.2 >= self.depth() {
            panic!("Out of index access");
        } else {
            &mut self.0[index]
        }
    }
}

pub trait InChunk<const WIDTH: usize> {
    /// Calculate the chunk, defined by a 2d index, a position is in - given a fixed chunk width
    fn in_chunk(&self) -> IVec2;
}

impl<const WIDTH: usize> InChunk<WIDTH> for Vec3 {
    fn in_chunk(&self) -> IVec2 {
        IVec2::new(
            self.x.div_euclid(WIDTH as f32) as i32,
            self.z.div_euclid(WIDTH as f32) as i32,
        )
    }
}

impl<const WIDTH: usize> InChunk<WIDTH> for IVec3 {
    fn in_chunk(&self) -> IVec2 {
        InChunk::<{ WIDTH }>::in_chunk(&self.as_f32())
    }
}

#[derive(Clone)]
pub struct GameChunk {
    pub voxel: Box<VoxelChunk<Block>>,
    pub index: IVec2,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Face {
    Top = 0,
    Bottom = 1,
    Left = 2,
    Right = 3,
    Front = 4,
    Back = 5,
}

/// The the four vertices that make up a cube face
/// When rendering with two triangle faces first two or the last two can be used as the common
/// points
#[inline]
fn quad_to_points(index: (i32, i32, i32), face: Face) -> [[f32; 3]; 4] {
    let (plane_offset, component_a, component_b) = match face {
        Face::Top => (-Vec3::Y, Vec3::X, Vec3::Z),
        Face::Bottom => (Vec3::ZERO, Vec3::Z, Vec3::X),
        Face::Front => (Vec3::ZERO, Vec3::Y, Vec3::Z),
        Face::Back => (-Vec3::X, Vec3::Z, Vec3::Y),
        Face::Left => (Vec3::ZERO, Vec3::X, Vec3::Y),
        Face::Right => (-Vec3::Z, Vec3::Y, Vec3::X),
    };
    let c = Vec3::new(index.0 as f32, index.1 as f32, index.2 as f32) - plane_offset;
    [
        c.into(),
        (c + component_a + component_b).into(),
        (c + component_a).into(),
        (c + component_b).into(),
    ]
}

pub trait Meshable {
    const FACES: [Face; 6] = [
        Face::Top,
        Face::Bottom,
        Face::Front,
        Face::Back,
        Face::Left,
        Face::Right,
    ];

    fn build(&self) -> HashMap<MeshGroup, Option<Mesh>>;
}

impl Meshable for GameChunk {
    fn build(&self) -> HashMap<MeshGroup, Option<Mesh>> {
        #[derive(Default)]
        struct BlockMesh {
            positions: Vec<[f32; 3]>,
            normals: Vec<[f32; 3]>,
            uvs: Vec<[f32; 2]>,
            indices: Vec<u32>,
            layer: Vec<u32>,
            index_counter: u32,
        }

        let mut block_meshes: HashMap<MeshGroup, BlockMesh> = HashMap::new();
        // Tracks blocktypes that have no mesh
        let mut non_existent: HashSet<MeshGroup> = blocks::EXCEPT_NONE_MESH_GROUP_SET.clone();

        for (idx, block) in self.voxel.indexed_iter() {
            let mesh_group = blocks::properties(block).mesh_group;

            if mesh_group != MeshGroup::None {
                non_existent.remove(&mesh_group);

                let e: &mut BlockMesh =
                    block_meshes.entry(mesh_group).or_insert_with(|| BlockMesh {
                        ..Default::default()
                    });

                let iidx = (idx.0 as i32, idx.1 as i32, idx.2 as i32);

                let tex_ids = blocks::BLOCK_TEX_ID.get(block).unwrap_or_else(|| {
                    warn!("Block `{:?}` has no texture id", block);
                    &[0; 6]
                });
                for face in Self::FACES.iter() {
                    let normal: [i32; 3] = match face {
                        Face::Top => [0, 1, 0],
                        Face::Bottom => [0, -1, 0],
                        Face::Front => [-1, 0, 0],
                        Face::Back => [1, 0, 0],
                        Face::Left => [0, 0, -1],
                        Face::Right => [0, 0, 1],
                    };
                    // Only add visible faces to the mesh
                    if blocks::MeshGroup::None
                        == self
                            .voxel
                            .safe_get(iidx.0 + normal[0], iidx.1 + normal[1], iidx.2 + normal[2])
                            .map_or(MeshGroup::None, |x| blocks::properties(x).mesh_group)
                    {
                        e.positions.extend(quad_to_points(iidx, *face).iter());

                        let normal = [normal[0] as f32, normal[1] as f32, normal[2] as f32];

                        e.normals.extend_from_slice(&[normal; 4]);
                        e.layer.extend_from_slice(&[tex_ids[*face as usize]; 4]);

                        let uv = if [Face::Top, Face::Front, Face::Right].contains(face) {
                            &[[0.0, 0.0], [1.0, -1.0], [0.0, -1.0], [1.0, 0.0]]
                        } else {
                            /*if [Face::Bottom, Face::Back, Face::Left].contains(face)*/
                            &[[0.0, 1.0], [-1.0, 0.0], [-1.0, 1.0], [0.0, 0.0]]
                        };

                        e.uvs.extend_from_slice(uv);

                        let c: u32 = e.index_counter;
                        // First triangle
                        e.indices.push(c);
                        e.indices.push(c + 1);
                        e.indices.push(c + 2);
                        // Second triangle
                        e.indices.push(c + 1);
                        e.indices.push(c);
                        e.indices.push(c + 3);

                        e.index_counter += 4;
                    }
                }
            }
        }

        let mut m: HashMap<MeshGroup, Option<Mesh>> = block_meshes
            .into_iter()
            .map(|(block_type, mesh_components)| {
                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, mesh_components.positions);
                mesh.set_attribute(
                    bevy::prelude::Mesh::ATTRIBUTE_NORMAL,
                    mesh_components.normals,
                );
                mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, mesh_components.uvs);
                mesh.set_attribute("Vertex_Layer", mesh_components.layer);
                mesh.set_indices(Some(bevy::render::mesh::Indices::U32(
                    mesh_components.indices,
                )));

                (block_type, Some(mesh))
            })
            .collect();

        non_existent.iter().for_each(|non_existent_block| {
            m.insert(*non_existent_block, None);
        });
        m
    }
}

/// Convert a chunk and voxel index to absolute world coordinates
pub fn index_to_absolut<const WIDTH: usize>(chunk: IVec2, index: UVec3) -> IVec3 {
    IVec3::from([
        chunk.x * WIDTH as i32 + index.x as i32,
        index.y as i32,
        chunk.y * WIDTH as i32 + index.z as i32,
    ])
}

/// Convert absolute world coordinates, given as Vec3 (f32), to the corresponding chunk and voxel index
pub fn absolut_to_index<const WIDTH: usize>(position: &Vec3) -> (IVec2, UVec3) {
    let chunk = InChunk::<{ WIDTH }>::in_chunk(position);
    (
        chunk,
        UVec3::new(
            (position.x - (chunk.x * WIDTH as i32) as f32) as u32,
            (position.y) as u32,
            (position.z - (chunk.y * WIDTH as i32) as f32) as u32,
        ),
    )
}

/// Convert absolute world coordinates, given as IVec3 (i32), to the corresponding chunk and voxel index
pub fn absolut_to_index_i32<const WIDTH: usize>(position: &IVec3) -> (IVec2, UVec3) {
    let chunk = InChunk::<{ defaults::CHUNK_WIDTH }>::in_chunk(position);
    (
        chunk,
        UVec3::new(
            (position.x - (chunk.x * defaults::CHUNK_WIDTH as i32) as i32) as u32,
            (position.y) as u32,
            (position.z - (chunk.y * defaults::CHUNK_WIDTH as i32) as i32) as u32,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_chunk() {
        const WIDTH: usize = 5;
        let cases: [(Vec3, IVec2); 20] = [
            (Vec3::new(0.0, 0.0, 0.0), IVec2::new(0, 0)),
            (Vec3::new(-0.1, 0.0, -0.001), IVec2::new(-1, -1)),
            (Vec3::new(0.0, 6.0, 0.0), IVec2::new(0, 0)),
            (Vec3::new(1.0, 0.0, 1.0), IVec2::new(0, 0)),
            (Vec3::new(4.999, -3.0, 4.999), IVec2::new(0, 0)),
            (Vec3::new(5.01, 1.0, 5.0), IVec2::new(1, 1)),
            (Vec3::new(9.999, -1.0, 9.999), IVec2::new(1, 1)),
            (Vec3::new(25.01, 1.0, 24.99), IVec2::new(5, 4)),
            (Vec3::new(-4.99, -12.0, 0.0), IVec2::new(-1, 0)),
            (Vec3::new(-5.01, -12.0, 0.0), IVec2::new(-2, 0)),
            (Vec3::new(-9.99, -12.0, -3.5), IVec2::new(-2, -1)),
            (Vec3::new(-15.01, -12.0, 6.0), IVec2::new(-4, 1)),
            (Vec3::new(5.0, 0.0, 10.0), IVec2::new(1, 2)),
            (Vec3::new(4.0, 0.0, 9.0), IVec2::new(0, 1)),
            (Vec3::new(-5.1, 0.0, 0.0), IVec2::new(-2, 0)),
            (Vec3::new(-4.9, 0.0, 0.0), IVec2::new(-1, 0)),
            (Vec3::new(-5.0, 0.0, 0.0), IVec2::new(-1, 0)),
            (Vec3::new(-10.1, 0.0, 0.0), IVec2::new(-3, 0)),
            (Vec3::new(-9.9, 0.0, 0.0), IVec2::new(-2, 0)),
            (Vec3::new(-10.0, 0.0, 0.0), IVec2::new(-2, 0)),
        ];
        for (position, ref_chunk) in cases.iter() {
            assert_eq!(
                InChunk::<{ WIDTH }>::in_chunk(position),
                *ref_chunk,
                "position = {}",
                position
            );
        }
    }
}
