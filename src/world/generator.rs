use super::*;
use bevy::prelude::*;
use noise::NoiseFn;
use noise::*;

pub trait WorldGenerator {
    /// Statelessly generate a chunk
    ///
    /// # Example
    /// ```
    /// fn chunk(at: IVec2, seed: u32) -> GameChunk {
    ///     const max_height: usize = 3;
    ///     assert!(max_height < defaults::CHUNK_HEIGHT);
    ///
    ///     let mut voxels = Box::new(VoxelChunk::air(defaults::CHUNK_SHAPE));
    ///     for x in 0..defaults::CHUNK_WIDTH {
    ///         for z in 0..defaults::CHUNK_WIDTH {
    ///            for y in 0..max_height {
    ///                voxels[(x, y, z)] = Blocks::Dirt;
    ///            }
    ///         }
    ///     }
    /// }
    /// ```
    fn chunk(at: IVec2, seed: u32) -> GameChunk;
}

pub struct BasicWorld;
impl WorldGenerator for BasicWorld {
    /// A basic procedural world generation algorithm. Note that this implementation has no
    /// philosophy behind it and was tuned to make the end-result look okay
    fn chunk(at: IVec2, seed: u32) -> GameChunk {
        let level_dirt = RidgedMulti::new().set_seed(seed);
        let level_dirt = ScalePoint::new(level_dirt).set_scale(0.01);
        let level_dirt_power_const = Constant::new(1.0);
        let level_dirt = Power::<[f64; 2]>::new(&level_dirt, &level_dirt_power_const);
        let level_dirt_offset = Constant::new(1.0);
        let level_dirt = Add::new(&level_dirt, &level_dirt_offset);
        let level_dirt = ScaleBias::new(&level_dirt)
            .set_scale(defaults::CHUNK_HEIGHT as f64 / 4.0)
            .set_bias(0.0);

        let level_grass = RidgedMulti::new().set_seed(seed.wrapping_add(1));
        let level_grass = ScalePoint::new(level_grass).set_scale(0.001);
        let level_grass_power_const = Constant::new(1.0);
        let level_grass = Power::<[f64; 2]>::new(&level_grass, &level_grass_power_const);
        let level_grass_offset = Constant::new(0.1);
        let level_grass = Add::new(&level_grass, &level_grass_offset);
        let level_grass = ScaleBias::new(&level_grass)
            .set_scale(defaults::CHUNK_HEIGHT as f64 / 4.0)
            .set_bias(0.0);

        let mix_nd = OpenSimplex::new().set_seed(seed);
        let mix_nd = ScalePoint::new(mix_nd).set_scale(0.006);
        let mix_nd_offset = Constant::new(0.6);
        let mix_nd_exp = Constant::new(3.0);
        let mix_nd = Add::new(&mix_nd, &mix_nd_offset);
        let mix_nd = Power::new(&mix_nd, &mix_nd_exp);
        let mix_nd2 = Perlin::new().set_seed(seed.wrapping_add(7));
        let mix_nd2 = ScalePoint::new(&mix_nd2).set_scale(0.006);
        let mix_nd2_offset = Constant::new(0.8);
        let mix_nd2 = Add::new(&mix_nd2, &mix_nd2_offset);
        let mix_nd_dithering = Perlin::new();
        let mix_nd_dithering = ScalePoint::new(mix_nd_dithering).set_scale(0.6);
        let mix_nd_dithering = ScaleBias::new(&mix_nd_dithering)
            .set_scale(0.1)
            .set_bias(-0.05);
        let mix_nd = Multiply::new(&mix_nd, &mix_nd2);
        let mix_nd = Clamp::new(&mix_nd).set_bounds(0.0, 1.0);
        let mix_nd_dithered = Add::new(&mix_nd, &mix_nd_dithering);
        let mix_nd_dithered = Clamp::new(&mix_nd_dithered).set_bounds(0.0, 1.0);

        let mix_nm = OpenSimplex::new().set_seed(seed.wrapping_add(3));
        let mix_nm = ScalePoint::new(mix_nm).set_scale(0.006);
        let mix_nm_offset = Constant::new(0.1);
        let mix_nm = Add::new(&mix_nm, &mix_nm_offset);
        let mix_nm = Clamp::new(&mix_nm).set_bounds(0.0, 1.0);

        let height_mountains = Perlin::new().set_seed(seed);
        let height_mountains = ScaleBias::<[f64; 2]>::new(&height_mountains)
            .set_scale(1.0)
            .set_bias(0.0);
        let height_mountains = ScalePoint::new(height_mountains).set_scale(0.05);

        let height_mountains2 = Perlin::new().set_seed(seed.wrapping_add(5));
        let height_mountains2 = ScaleBias::<[f64; 2]>::new(&height_mountains2)
            .set_scale(0.1)
            .set_bias(-0.05);
        let height_mountains2 = ScalePoint::new(height_mountains2).set_scale(0.15);
        let height_mountains = Add::new(&height_mountains, &height_mountains2);
        let height_mountains = ScaleBias::<[f64; 2]>::new(&height_mountains)
            .set_scale(defaults::CHUNK_HEIGHT as f64 / 1.0)
            .set_bias(0.0);

        let height_dirt = Perlin::new().set_seed(seed);
        let height_dirt = ScaleBias::<[f64; 2]>::new(&height_dirt)
            .set_scale(1.0)
            .set_bias(0.0);
        let height_dirt = ScalePoint::new(height_dirt).set_scale(0.006);

        let height_dirt2 = Perlin::new().set_seed(seed.wrapping_add(1));
        let height_dirt2 = ScaleBias::<[f64; 2]>::new(&height_dirt2)
            .set_scale(1.0)
            .set_bias(0.0);
        let height_dirt2 = ScalePoint::new(height_dirt2).set_scale(0.013);

        let height_sand = Perlin::new().set_seed(seed.wrapping_add(2));
        let height_sand = ScalePoint::new(height_sand).set_scale(0.003);

        let height_dirt = Multiply::new(&height_dirt, &height_dirt2);
        let height_dirt = ScaleBias::<[f64; 2]>::new(&height_dirt)
            .set_scale(defaults::CHUNK_HEIGHT as f64 / 2.0)
            .set_bias(0.0);
        let height_sand = ScaleBias::<[f64; 2]>::new(&height_sand)
            .set_scale(defaults::CHUNK_HEIGHT as f64 / 2.0)
            .set_bias(0.0);
        let height = Blend::new(&height_dirt, &height_mountains, &mix_nm);
        let height = Blend::new(&height, &height_sand, &mix_nd);

        let height_offset = Constant::new(defaults::CHUNK_HEIGHT as f64 / 4.0);
        let add = Add::new(&height, &height_offset);
        let clamp = Clamp::new(&add).set_bounds(2.0, defaults::CHUNK_HEIGHT as f64);
        let output = &clamp;

        let tree_distr = SuperSimplex::new().set_seed(seed.wrapping_add(6));
        let tree_distr = ScaleBias::new(&tree_distr).set_scale(1.0).set_bias(0.0);
        let tree_distr = ScalePoint::new(tree_distr).set_scale(0.15);
        let tree_distr_exp = Constant::new(1.3);
        let tree_distr = Power::new(&tree_distr, &tree_distr_exp);
        let tree_distr = Clamp::new(&tree_distr).set_bounds(0.0, 1.0);

        let height_tree = Perlin::new().set_seed(seed.wrapping_add(13));
        let height_tree = ScaleBias::new(&height_tree).set_scale(3.0).set_bias(3.0);
        let height_tree = ScalePoint::new(&height_tree).set_scale(1.1);

        let mut voxels = Box::new(VoxelChunk::air(defaults::CHUNK_SHAPE));
        let chunk_offset_x: f64 = at.x as f64 * defaults::CHUNK_WIDTH as f64;
        let chunk_offset_y: f64 = at.y as f64 * defaults::CHUNK_WIDTH as f64;
        macro_rules! offset {
            ($x:expr, $z:expr) => {
                [($x as f64 + chunk_offset_x), ($z as f64 + chunk_offset_y)]
            };
        }
        macro_rules! offset3 {
            ($x:expr, $y:expr, $z:expr) => {
                [
                    ($x as f64 + chunk_offset_x),
                    $y as f64,
                    ($z as f64 + chunk_offset_y),
                ]
            };
        }
        for x in 0..defaults::CHUNK_WIDTH {
            for z in 0..defaults::CHUNK_WIDTH {
                let height =
                    (output.get(offset!(x, z)) as usize).clamp(0, defaults::CHUNK_HEIGHT - 1);

                let mix_val = mix_nd_dithered.get(offset!(x, z));
                for y in 0..height {
                    voxels[(x, y, z)] = if mix_val <= 0.5 {
                        if height as f64 >= level_grass.get(offset!(x, z)) && y + 1 == height {
                            BlockType::Grass
                        } else if height as f64 >= level_dirt.get(offset!(x, z)) {
                            BlockType::Dirt
                        } else {
                            BlockType::Gravel
                        }
                    } else {
                        BlockType::Sand
                    };
                }
                if mix_val <= 0.5 {
                    for attempt in 0..3 {
                        let val = tree_distr.get(offset!(x + attempt * 2000, z + attempt * 120));
                        if val >= 0.96
                            && ![0, 1, defaults::CHUNK_WIDTH - 2, defaults::CHUNK_WIDTH - 1]
                                .contains(&x)
                            && ![0, 1, defaults::CHUNK_WIDTH - 2, defaults::CHUNK_WIDTH - 1]
                                .contains(&z)
                        {
                            let height_tree = height_tree.get(offset!(x, z)) as usize;
                            let leaves = Fbm::new()
                                .set_seed(
                                    seed.wrapping_add(x.rem_euclid(u32::MAX as usize) as u32)
                                        .wrapping_add(
                                            (z.wrapping_mul(2)).rem_euclid(u32::MAX as usize)
                                                as u32,
                                        ),
                                )
                                .set_frequency(2.0)
                                .set_lacunarity(2.0)
                                .set_octaves(15);
                            let leaves = ScalePoint::new(&leaves).set_scale(0.1);
                            let leaves = ScaleBias::new(&leaves).set_scale(0.5).set_bias(0.9);
                            let leaves = Clamp::new(&leaves).set_bounds(0.0, 1.0);
                            for y in
                                height..(height + height_tree).clamp(0, defaults::CHUNK_HEIGHT - 1)
                            {
                                voxels[(x, y, z)] = BlockType::Wood;
                            }

                            let lower_height =
                                (height + height_tree).clamp(0, defaults::CHUNK_HEIGHT - 1);
                            let upper_height =
                                (height + height_tree + 4).clamp(0, defaults::CHUNK_HEIGHT - 1);
                            for y in lower_height..upper_height {
                                for a in -4..4 {
                                    for b in -4..4 {
                                        if ((a as f32).powi(2)
                                            + (y as f32
                                                - lower_height as f32
                                                - (upper_height as f32 - lower_height as f32)
                                                    / 3.0)
                                                .powi(2)
                                            + (b as f32).powi(2))
                                        .sqrt()
                                            / (3.0f32.powi(2) * 3.0).sqrt()
                                            * (leaves.get(offset3!(
                                                x as i32 + a,
                                                y * 2,
                                                z as i32 + b
                                            ))
                                                as f32)
                                            < 0.4
                                        {
                                            voxels[(
                                                (x as i32 - a)
                                                    .clamp(0, defaults::CHUNK_WIDTH as i32 - 1)
                                                    as usize,
                                                y,
                                                (z as i32 - b)
                                                    .clamp(0, defaults::CHUNK_WIDTH as i32 - 1)
                                                    as usize,
                                            )] = BlockType::Leaves;
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                    voxels[(x, 0, z)] = BlockType::Cobble;
                }
            }
        }
        GameChunk {
            voxel: voxels,
            index: at,
        }
    }
}
