use crate::level::db_interface::bedrock_key::ChunkKey;
use bedrockrs_shared::world::dimension::Dimension;
use std::collections::HashSet;
use std::ops::Range;
use std::path::Path;
use vek::Vec2;

pub struct DatabaseBatchHolder {
    collective: Vec<u8>,
    key_range: Range<usize>,
    data_range: Range<usize>,
}

impl DatabaseBatchHolder {
    pub fn new(collective: Vec<u8>, key_range: Range<usize>, data_range: Range<usize>) -> Self {
        Self {
            collective,
            key_range,
            data_range,
        }
    }

    pub fn key(&self) -> &[u8] {
        &self.collective[self.key_range.clone()]
    }

    pub fn data(&self) -> &[u8] {
        &self.collective[self.data_range.clone()]
    }
}

pub trait RawWorldTrait: Sized {
    type Err;

    type UserState;

    fn write_bytes_to_key(
        &mut self,
        chunk_info: ChunkKey,
        chunk_bytes: &[u8],
        state: &mut Self::UserState,
    ) -> Result<(), Self::Err>;

    fn get_bytes_from_key(
        &mut self,
        chunk_info: ChunkKey,
        state: &mut Self::UserState,
    ) -> Result<Option<Vec<u8>>, Self::Err>;

    fn delete_bytes_at_key(
        &mut self,
        chunk_info: ChunkKey,
        state: &mut Self::UserState,
    ) -> Result<(), Self::Err>;

    fn set_subchunk_raw(
        &mut self,
        chunk_info: ChunkKey,
        chunk_bytes: &[u8],
        state: &mut Self::UserState,
    ) -> Result<(), Self::Err> {
        self.write_bytes_to_key(chunk_info, chunk_bytes, state)
    }

    fn get_subchunk_raw(
        &mut self,
        chunk_info: ChunkKey,
        state: &mut Self::UserState,
    ) -> Result<Option<Vec<u8>>, Self::Err> {
        self.get_bytes_from_key(chunk_info, state)
    }

    fn chunk_exists(
        &mut self,
        chunk_info: ChunkKey,
        state: &mut Self::UserState,
    ) -> Result<bool, Self::Err> {
        Ok(self.get_bytes_from_key(chunk_info, state)?.is_some())
    }

    fn write_subchunk_batch(
        &mut self,
        subchunk_batch_info: Vec<(ChunkKey, Vec<u8>)>,
        state: &mut Self::UserState,
    ) -> Result<(), Self::Err>;

    fn write_subchunk_marker_batch(
        &mut self,
        subchunk_batch_info: Vec<ChunkKey>,
        state: &mut Self::UserState,
    ) -> Result<(), Self::Err>;

    fn mark_exist_chunk(
        &mut self,
        chunk_info: ChunkKey,
        state: &mut Self::UserState,
    ) -> Result<(), Self::Err> {
        self.write_bytes_to_key(chunk_info, &[], state)
    }

    fn build_key(key: &ChunkKey) -> Vec<u8>;

    fn new(
        path: Box<Path>,
        create_if_missing: bool,
        state: &mut Self::UserState,
    ) -> Result<Self, Self::Err>;

    fn close(&mut self) -> Result<(), Self::Err>;

    fn generated_chunks(
        &mut self,
        state: &mut Self::UserState,
    ) -> Result<HashSet<(Dimension, Vec2<i32>)>, Self::Err>;

    fn delete_chunk(
        &mut self,
        xz: Vec2<i32>,
        dimension: Dimension,
        subchunk_range: Vec2<i8>,
        state: &mut Self::UserState,
    ) -> Result<(), Self::Err> {
        for y in subchunk_range.x..=subchunk_range.y {
            self.delete_subchunk(xz, dimension, y, state)?
        }

        self.delete_bytes_at_key(ChunkKey::chunk_marker(xz, dimension), state)?;

        Ok(())
    }

    fn delete_subchunk(
        &mut self,
        xz: Vec2<i32>,
        dimension: Dimension,
        y: i8,
        state: &mut Self::UserState,
    ) -> Result<(), Self::Err> {
        self.delete_bytes_at_key(ChunkKey::new_subchunk(xz, dimension, y), state)
    }
}
