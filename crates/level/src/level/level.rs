use crate::level::db_interface::bedrock_key::ChunkKey;
use crate::level::file_interface::RawWorldTrait;
use crate::level::sub_chunk::{
    SerDeStoreRef, SerDeTrait, SubChunk, SubChunkDecoder, SubChunkEncoder, SubChunkSerDe,
    SubChunkTrait, SubChunkTraitExtended, SubChunkTransition,
};
use crate::level::world_block::{BlockTransition, LevelBlock};
use bedrockrs_shared::world::dimension::Dimension;
use std::collections::hash_set::Iter;
use std::collections::HashSet;
use std::fmt::Debug;
use std::path::Path;
use vek::{Vec2, Vec3};

/// This is used when filtering chunks.
/// `ChunkSelectionFilter::Dimension` is used to just check if the dimension is the same.
/// `ChunkSelectionFilter::Filter` is used to perform more complex logic on the chunk to detect if it should be included
pub enum ChunkSelectionFilter {
    Dimension(Dimension),
    Filter(Box<dyn FnMut(Dimension, Vec2<i32>) -> bool>),
}

/// This is used when filtering sub chunks.
///
/// `SubchunkSelectionFilter::Dimension` is used to just check if the dimension is the same.
///
/// `SubchunkSelectionFilter::Filter`
/// is used to perform more complex logic on the chunk to detect if it should be included
pub enum SubchunkSelectionFilter {
    Dimension(Dimension),
    Filter(Box<dyn FnMut(Dimension, i8, Vec2<i32>) -> bool>),
}

impl ChunkSelectionFilter {
    pub fn valid(&mut self, chunk_dim: Dimension, pos: Vec2<i32>) -> bool {
        match self {
            ChunkSelectionFilter::Dimension(dim) => dim == &chunk_dim,
            ChunkSelectionFilter::Filter(func) => func(chunk_dim, pos),
        }
    }
}

#[derive(Debug)]
pub struct LevelConfiguration {
    pub sub_chunk_range: Vec2<i8>,
    pub rw_cache: bool,
    pub create_db_if_missing: bool,
}

impl Default for LevelConfiguration {
    fn default() -> Self {
        Self {
            sub_chunk_range: (-4, 20).into(),
            rw_cache: false,
            create_db_if_missing: false,
        }
    }
}

#[derive(Debug)]
pub struct SetBlockConfig<Block: BlockTransition> {
    pub position: Vec3<i32>,
    pub block: Block,
    pub dim: Dimension,
    pub layer: u8,
}

#[derive(Debug)]
pub struct GetBlockConfig {
    pub position: Vec3<i32>,
    pub dim: Dimension,
    pub layer: u8,
}

#[derive(Debug)]
pub struct FillConfig<Block: BlockTransition> {
    pub from: Vec3<i32>,
    pub to: Vec3<i32>,
    pub block: Block,
    pub dim: Dimension,
    pub layer: u8,
    pub data_version: u8,
}

#[allow(dead_code)]
pub struct Level<UserWorldInterface: RawWorldTrait>
where
    <UserWorldInterface as RawWorldTrait>::Err: Debug,
{
    db: UserWorldInterface,
    config: LevelConfiguration,
    chunk_existence: HashSet<(Dimension, Vec2<i32>)>,
}

#[allow(dead_code)]
impl<UserWorldInterface: RawWorldTrait> Level<UserWorldInterface>
where
    <UserWorldInterface as RawWorldTrait>::Err: Debug,
{
    /// Simple function used to open the world
    pub fn open<ConstructionInformation>(
        path: Box<Path>,
        config: LevelConfiguration,
        information: &mut ConstructionInformation,
    ) -> Result<Self, UserWorldInterface::Err>
    where
        UserWorldInterface: RawWorldTrait<ConstructionInformation = ConstructionInformation>,
    {
        let db = {
            let val =
                UserWorldInterface::open(path.clone(), config.create_db_if_missing, information);
            if let Ok(v) = val {
                Ok(v)
            } else {
                UserWorldInterface::open(
                    {
                        let mut buff = path.into_path_buf();
                        buff.push("db");
                        buff.into_boxed_path()
                    },
                    config.create_db_if_missing,
                    information,
                )
            }
        }?;
        let mut this = Self {
            db,
            config,
            chunk_existence: HashSet::new(),
        };
        this.chunk_existence = this.db.generated_chunks()?;
        Ok(this)
    }

    /// Provides a mut ref to the underlying database implementation
    pub fn underlying_world_interface(&mut self) -> &mut UserWorldInterface {
        &mut self.db
    }

    /// Fills blocks into the world in the boundaries given.
    /// This function works on a global scale meaning it works on/over sub chunk boundaries
    /// This uses the default Encoders and Decoders. If custom ones are needed use [`Level::fill_ex`].
    pub fn fill<BlockType: BlockTransition>(
        &mut self,
        config: FillConfig<BlockType>,
    ) -> Result<
        (),
        LevelFilLError<
            UserWorldInterface::Err,
            <SubChunkSerDe as SubChunkDecoder>::Err,
            <SubChunkSerDe as SubChunkEncoder>::Err,
            <SubChunk as SubChunkTrait>::Err,
        >,
    > {
        self.fill_ex(config, &mut SubChunkSerDe, &mut SubChunkSerDe)
    }

    /// Fills blocks into the world in the boundaries given.
    /// This function works on a global scale meaning it works on/over sub chunk boundaries
    /// This uses custom Encoders and Decoders
    pub fn fill_ex<BlockType: BlockTransition, Decoder: SubChunkDecoder, Encoder: SubChunkEncoder>(
        &mut self,
        config: FillConfig<BlockType>,
        decoder: &mut Decoder,
        encoder: &mut Encoder,
    ) -> Result<
        (),
        LevelFilLError<
            UserWorldInterface::Err,
            Decoder::Err,
            Encoder::Err,
            <SubChunk as SubChunkTrait>::Err,
        >,
    >
    where
        Decoder::Err: Debug,
        Encoder::Err: Debug,
    {
        // This gets the `to` and `from` into a state where `from` will always be the bottom left of the selection and `to` will be the top right
        let (from, to) = bounds_left_optimized(config.to, config.from);

        let (min, local_min) = LevelBlock::block_pos_to_sub_chunk(from);
        let (max, local_max) = LevelBlock::block_pos_to_sub_chunk(to);

        let block = config.block.into_transition();

        println!("{min} {max}");
        for x in min.x..=max.x {
            for z in min.z..=max.z {
                for y in min.y..=max.y {
                    println!("{x} {y} {z}");
                    if in_range(min.x, max.x, x)
                        && in_range(min.y, max.y, y)
                        && in_range(min.z, max.z, z)
                    {
                        // Since this whole sub chunk must be replaced we will just create a new one to cut down on the overhead of reading it
                        let pos = (x, y, z).into();

                        let data =
                            SubChunkTransition::full(pos, config.data_version, block.clone());

                        self.set_sub_chunk_raw(encoder, data, pos, config.dim)
                            .map_err(|err| LevelFilLError::from_encode(err.into()))?;
                    } else {
                        // Now we need to find where this sub chunk lies in the boundary
                        println!("Called");

                        let invert_x = x == max.x;
                        let invert_y = y == max.y;
                        let invert_z = z == max.z;

                        let x_range = if invert_x {
                            0..=local_max.x
                        } else {
                            local_min.x..=15
                        };

                        let y_range = if invert_y {
                            0..=local_max.y
                        } else {
                            local_min.y..=15
                        };

                        let z_range = if invert_z {
                            0..=local_max.z
                        } else {
                            local_min.z..=15
                        };

                        // This is slightly convoluted to allow for fewer allocations,
                        // by generating a sub chunk directly if it didn't exist in the database
                        // [`Result::unwrap_or_else`] is used over [`Result::unwrap_or`] to bypass the overhead of creating the default even when it's not needed
                        let mut data = self
                            .get_sub_chunk_raw::<SubChunk, Decoder>(
                                (x, y, z).into(),
                                config.dim,
                                decoder,
                            )
                            .map_err(|err| LevelFilLError::from_decode(err.into()))?
                            .map(|transition| {
                                SubChunk::decode_from_transition(transition, config.dim, &mut ())
                                    .unwrap() // This is a safe unwrap because `SubChunk::decode_from_translation` never fails
                            })
                            .unwrap_or_else(|| {
                                {
                                    SubChunk::empty((x, y, z).into(), config.dim).to_init()
                                }
                            });

                        for x in x_range {
                            for z in z_range.clone() {
                                for y in y_range.clone() {
                                    data.set_block((x, y, z).into(), block.clone())
                                        .map_err(|err| LevelFilLError::SubChunkError(err))?;
                                }
                            }
                        }

                        let translation = data.to_transition(&mut ()).unwrap(); // Safe because this can't fail
                        dbg!(&translation);
                        self.set_sub_chunk_raw(encoder, translation, (x, y, z).into(), config.dim)
                            .map_err(|err| LevelFilLError::from_encode(err.into()))?;
                    }
                }
            }
        }
        Ok(())
    }

    /// # Warning
    /// This function is incredibly expensive to call due to having to deserialize the sub chunk. Only call this for 1 off cases of getting blocks
    ///
    /// # Description
    /// Gets a block at a specific position in the world.
    /// # Info
    /// This function uses the default decoder implementation.
    /// If a custom Decoder is needed please use [`Level::get_block_ex`].
    pub fn get_block<BlockType: BlockTransition>(
        &mut self,
        config: GetBlockConfig,
    ) -> Result<
        Option<BlockType>,
        SubChunkReadWriteError<
            UserWorldInterface::Err,
            <SubChunkSerDe as SubChunkEncoder>::Err,
            <SubChunk as SubChunkTrait>::Err,
        >,
    > {
        self.get_block_ex::<BlockType, SubChunkSerDe>(
            config.position,
            config.dim,
            config.layer,
            &mut SubChunkSerDe,
        )
    }

    /// # Warning
    /// This function is incredibly expensive to call due to having to deserialize the sub chunk. Only call this for 1 off cases of getting blocks
    ///
    /// # Description
    /// Gets a block at a specific position in the world.
    /// # Info
    /// This function uses a custom decoder.
    /// If a custom layer is not needed and default decoding is fine use [`Level::get_block`].
    pub fn get_block_ex<BlockType: BlockTransition, Decoder: SubChunkDecoder>(
        &mut self,
        pos: Vec3<i32>,
        dim: Dimension,
        layer: u8,
        decoder: &mut Decoder,
    ) -> Result<
        Option<BlockType>,
        SubChunkReadWriteError<
            UserWorldInterface::Err,
            Decoder::Err,
            <SubChunk as SubChunkTrait>::Err,
        >,
    >
    where
        Decoder::Err: Debug,
    {
        let (sub_chunk_pos, local_pos) = LevelBlock::block_pos_to_sub_chunk(pos);

        let data = if let Some(data) = self.get_sub_chunk::<SubChunk, Decoder>(
            sub_chunk_pos,
            dim,
            SerDeStoreRef::new(decoder, &mut ()),
        )? {
            data
        } else {
            let mut out = SubChunk::empty(sub_chunk_pos, dim);

            if layer != 0 {
                for _ in 1..=layer {
                    out.add_sub_layer()
                }
            }
            out.set_active_layer(layer);

            out
        };

        Ok(data
            .get_block(local_pos)
            .cloned()
            .map(|ele| BlockType::from_transition(ele)))
    }

    /// # Warning
    /// This function is incredibly expensive to call due to having to deserialize and serialize the sub chunk. Only call this for 1 off cases of setting blocks
    ///
    /// # Description
    /// Sets a block at a specific position in the world.
    /// # Info
    /// This function uses the default decoder and encoder implementations.
    /// If a custom Decoder or Encoder is needed please use [`Level::set_block_ex`].
    pub fn set_block<BlockType: BlockTransition>(
        &mut self,
        config: SetBlockConfig<BlockType>,
    ) -> Result<
        (),
        SubChunkReadWriteError<
            UserWorldInterface::Err,
            <SubChunkSerDe as SubChunkEncoder>::Err,
            <SubChunk as SubChunkTrait>::Err,
        >,
    > {
        self.set_block_ex::<BlockType, SubChunkSerDe, SubChunkSerDe>(
            config.block,
            config.position,
            config.dim,
            config.layer,
            &mut SubChunkSerDe,
            &mut SubChunkSerDe,
        )
    }

    /// # Warning
    /// This function is incredibly expensive to call due to having to deserialize and serialize the sub chunk. Only call this for 1 off cases of setting blocks
    ///
    /// # Description
    /// Sets a block at a specific position in the world.
    /// # Info
    /// This function uses a custom Encoder and Decoder.
    /// If a custom layer is not needed and default Encoding and Decoding is fine use [`Level::set_block`].
    pub fn set_block_ex<
        BlockType: BlockTransition,
        Decoder: SubChunkDecoder,
        Encoder: SubChunkEncoder,
    >(
        &mut self,
        block: BlockType,
        pos: Vec3<i32>,
        dim: Dimension,
        layer: u8,
        decoder: &mut Decoder,
        encoder: &mut Encoder,
    ) -> Result<
        (),
        SubChunkReadWriteError<
            UserWorldInterface::Err,
            Encoder::Err,
            <SubChunk as SubChunkTrait>::Err,
        >,
    >
    where
        Decoder::Err: Debug,
        Encoder::Err: Debug,
    {
        let (sub_chunk_pos, local_pos) = LevelBlock::block_pos_to_sub_chunk(pos);

        let mut data = if let Ok(Some(data)) = self.get_sub_chunk::<SubChunk, Decoder>(
            sub_chunk_pos,
            dim,
            SerDeStoreRef::new(decoder, &mut ()),
        ) {
            data
        } else {
            let mut out = SubChunk::empty(sub_chunk_pos, dim);

            if layer != 0 {
                for _ in 1..=layer {
                    out.add_sub_layer()
                }
            }
            out.set_active_layer(layer);

            out
        };

        data.set_block(local_pos, block.into_transition())?;

        self.set_sub_chunk::<SubChunk, Encoder>(&data, SerDeStoreRef::new(encoder, &mut ()))
    }

    /// High level function to fetch a sub chunk that contains a specific block.
    /// If the sub chunk doesn't exist this will return None
    pub fn get_sub_chunk_block_position<SubChunkType: SubChunkTrait, Decoder: SubChunkDecoder>(
        &mut self,
        pos: Vec3<i32>,
        dim: Dimension,
        config: impl SerDeTrait<SerDe = Decoder, Info = SubChunkType::TransitionInformation>,
    ) -> Result<
        Option<SubChunkType>,
        SubChunkReadWriteError<UserWorldInterface::Err, Decoder::Err, SubChunkType::Err>,
    >
    where
        Decoder::Err: Debug,
        SubChunkType::Err: Debug,
    {
        let (sub_chunk_pos, _) = LevelBlock::block_pos_to_sub_chunk(pos);
        self.get_sub_chunk::<SubChunkType, Decoder>(sub_chunk_pos, dim, config)
    }

    /// High level function to fetch a sub chunk directly from the database.
    /// If the sub chunk doesn't exist this will return None
    pub fn get_sub_chunk<SubChunkType: SubChunkTrait, Decoder: SubChunkDecoder>(
        &mut self,
        pos: Vec3<i32>,
        dim: Dimension,
        mut config: impl SerDeTrait<SerDe = Decoder, Info = SubChunkType::TransitionInformation>,
    ) -> Result<
        Option<SubChunkType>,
        SubChunkReadWriteError<UserWorldInterface::Err, Decoder::Err, SubChunkType::Err>,
    >
    where
        Decoder::Err: Debug,
        SubChunkType::Err: Debug,
    {
        let ser = self.get_sub_chunk_raw::<SubChunkType, Decoder>(pos, dim, config.serde())?;

        let ser = match ser {
            None => return Ok(None),
            Some(e) => e,
        };

        Ok(Some(
            SubChunkType::decode_from_transition(ser, dim, config.info())
                .map_err(|ele| SubChunkReadWriteError::TranslationError(ele))?,
        ))
    }
    pub fn get_sub_chunk_raw<SubChunkType: SubChunkTrait, Decoder: SubChunkDecoder>(
        &mut self,
        pos: Vec3<i32>,
        dim: Dimension,
        decoder: &mut Decoder,
    ) -> Result<Option<SubChunkTransition>, SubChunkSerDeError<UserWorldInterface::Err, Decoder::Err>>
    where
        Decoder::Err: Debug,
    {
        let bytes = self
            .db
            .get_sub_chunk_raw(ChunkKey::new_sub_chunk(pos, dim))
            .map_err(|ele| SubChunkSerDeError::WorldError(ele))?;

        let bytes = match bytes {
            None => return Ok(None),
            Some(e) => e,
        };

        Ok(Some(
            decoder
                .decode_bytes_as_sub_chunk(&mut std::io::Cursor::new(bytes), (pos.x, pos.z).into())
                .map_err(|ele| SubChunkSerDeError::SerDeError(ele))?,
        ))
    }

    /// High level function to write a sub chunk directly into the database.
    /// Writes the sub chunk at its current dimension and position.
    /// If a dimension or position override is required please call [`Level::set_sub_chunk_ex`]
    pub fn set_sub_chunk<
        SubChunkType: SubChunkTraitExtended,
        SubChunkEncoderType: SubChunkEncoder,
    >(
        &mut self,
        sub_chunk: &SubChunkType,
        encode_data: impl SerDeTrait<
            Info = SubChunkType::TransitionInformation,
            SerDe = SubChunkEncoderType,
        >,
    ) -> Result<
        (),
        SubChunkReadWriteError<
            UserWorldInterface::Err,
            SubChunkEncoderType::Err,
            SubChunkType::Err,
        >,
    >
    where
        SubChunkEncoderType::Err: Debug,
        SubChunkType::Err: Debug,
    {
        self.set_sub_chunk_ex(
            sub_chunk,
            encode_data,
            sub_chunk.position(),
            sub_chunk.dimension(),
        )
    }

    /// Slightly lower level function to write a sub chunk directly into the database.
    /// Writes the sub chunk at the position and dimension provided.
    /// If you do not need to override this please use [`Level::set_sub_chunk`] instead
    pub fn set_sub_chunk_ex<
        SubChunkType: SubChunkTraitExtended,
        SubChunkEncoderType: SubChunkEncoder,
    >(
        &mut self,
        sub_chunk: &SubChunkType,
        mut encode_data: impl SerDeTrait<
            Info = SubChunkType::TransitionInformation,
            SerDe = SubChunkEncoderType,
        >,
        pos: Vec3<i32>,
        dim: Dimension,
    ) -> Result<
        (),
        SubChunkReadWriteError<
            UserWorldInterface::Err,
            SubChunkEncoderType::Err,
            SubChunkType::Err,
        >,
    >
    where
        SubChunkType::Err: Debug,
        SubChunkEncoderType::Err: Debug,
    {
        let intermediate = sub_chunk
            .to_transition(encode_data.info())
            .map_err(|ele| SubChunkReadWriteError::TranslationError(ele))?;
        Ok(self.set_sub_chunk_raw(encode_data.serde(), intermediate, pos, dim)?)
    }

    /// Lowest level setting function for a sub chunk.
    /// This function takes a transition and writes it into the database directly.
    /// This is a powerful function which can lead to invalid data being written into the database
    pub fn set_sub_chunk_raw<SubChunkEncoderType: SubChunkEncoder>(
        &mut self,
        encoder: &mut SubChunkEncoderType,
        intermediate: SubChunkTransition,
        pos: Vec3<i32>,
        dim: Dimension,
    ) -> Result<(), SubChunkSerDeError<UserWorldInterface::Err, SubChunkEncoderType::Err>>
    where
        SubChunkEncoderType::Err: Debug,
    {
        let bytes = encoder
            .write_as_bytes(intermediate, false)
            .map_err(|ele| SubChunkSerDeError::SerDeError(ele))?;

        self.db
            .set_sub_chunk_raw(ChunkKey::new_sub_chunk(pos, dim), &bytes)
            .map_err(|ele| SubChunkSerDeError::WorldError(ele))?;

        self.handle_exist((pos.x, pos.z).into(), dim);
        Ok(())
    }

    /// Used to delete a whole chunk from the world. This clears the data from the database. It only removes data in the configured range of the level.
    /// If clearing inside or outside the range which the level covers is needed see [`Level::remove_chunk_ex`]
    pub fn remove_chunk(
        &mut self,
        xz: Vec2<i32>,
        dimension: Dimension,
    ) -> Result<(), UserWorldInterface::Err> {
        self.remove_chunk_ex(xz, dimension, self.config.sub_chunk_range.clone())
    }

    /// Used to delete a chunk from the world. This clears data from the database. This removes all sub chunks in the range specified.
    /// If you do not need to delete only a section of information use [`Level::remove_chunk`]
    pub fn remove_chunk_ex(
        &mut self,
        xz: Vec2<i32>,
        dimension: Dimension,
        sub_chunk_range: Vec2<i8>,
    ) -> Result<(), UserWorldInterface::Err> {
        self.chunk_existence.remove(&(dimension, xz));
        self.db.delete_chunk(xz, dimension, sub_chunk_range)
    }

    /// Removes a sub chunk from the database
    pub fn remove_sub_chunk(
        &mut self,
        xyz: Vec3<i32>,
        dimension: Dimension,
    ) -> Result<(), UserWorldInterface::Err> {
        self.db.delete_sub_chunk(xyz, dimension)
    }

    /// Checks if a chunk exists and returns the result
    pub fn chunk_exists(&mut self, xz: Vec2<i32>, dimension: Dimension) -> bool {
        self.chunk_existence.contains(&(dimension, xz))
    }

    /// Closes the current level and saves all information to the DB
    pub fn close(mut self) -> Result<(), UserWorldInterface::Err> {
        self.close_internal()
    }

    /// Returns all chunks (in the form of its key) that exist in the world
    pub fn existence_chunks(&self) -> Iter<'_, (Dimension, Vec2<i32>)> {
        self.chunk_existence.iter()
    }

    /// This function WIPES all chunks that exist in the target dimension. This doesn't mean reset. This deletes them
    /// This function only effects sub chunks in the current level range. If you only want to delete sections of the chunk please call [`Level::clear_ex`]
    pub fn clear(&mut self, dim: Dimension) -> Result<(), UserWorldInterface::Err> {
        self.clear_ex(dim, self.config.sub_chunk_range)
    }

    /// This function WIPES all chunks that exist in the target dimension. This doesn't mean reset. This deletes them
    /// This function
    pub fn clear_ex(
        &mut self,
        dim: Dimension,
        sub_chunk_range: Vec2<i8>,
    ) -> Result<(), UserWorldInterface::Err> {
        self.get_chunk_keys(ChunkSelectionFilter::Dimension(dim))
            .drain(..)
            .try_for_each(|ele| self.remove_chunk_ex(ele, dim, sub_chunk_range))
    }

    /// Fetches all chunk keys that satisfy the filter's constraints
    pub fn get_chunk_keys(&mut self, mut filter: ChunkSelectionFilter) -> Vec<Vec2<i32>> {
        self.chunk_existence
            .iter()
            .filter_map(|(chunk_dim, pos)| {
                if filter.valid(chunk_dim.clone(), *pos) {
                    Some(*pos)
                } else {
                    None
                }
            })
            .collect()
    }

    // Internal functions which mainly handle DB layer interactions
    fn close_internal(&mut self) -> Result<(), UserWorldInterface::Err> {
        self.flush_existence_buffer()?;

        // Must come after all the other closing steps
        self.db.close()
    }

    fn handle_exist(&mut self, xz: Vec2<i32>, dim: Dimension) {
        self.chunk_existence.insert((dim, xz));
    }
    fn flush_existence_buffer(&mut self) -> Result<(), UserWorldInterface::Err> {
        for (dim, pos) in &self.chunk_existence {
            self.db
                .mark_exist_chunk(ChunkKey::chunk_marker(*pos, *dim))?
        }
        Ok(())
    }
}

impl<UserWorldInterface: RawWorldTrait> Drop for Level<UserWorldInterface>
where
    <UserWorldInterface as RawWorldTrait>::Err: Debug,
{
    fn drop(&mut self) {
        self.close_internal().unwrap();
    }
}

#[cfg(feature = "default-impl")]
#[allow(unused_imports)]
pub mod default_impl {
    use super::*;
    use crate::level::db_interface::rusty::RustyDBInterface;
    use crate::level::sub_chunk::{SerDeStore, SubChunk, SubChunkSerDe};
    use crate::level::world_block::LevelBlock;

    pub type LevelDbInterface = RustyDBInterface;
    pub type LevelDbError = <RustyDBInterface as RawWorldTrait>::Err;
    pub type BedrockLevel = Level<LevelDbInterface>;
}

use crate::level::error::{LevelFilLError, SubChunkReadWriteError, SubChunkSerDeError};
use crate::utility::miner::{bounds_left_optimized, in_range};
#[cfg(feature = "default-impl")]
pub use default_impl::*;
