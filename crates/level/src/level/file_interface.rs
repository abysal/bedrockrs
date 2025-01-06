use crate::level::db_interface::bedrock_key::ChunkKey;
use bedrockrs_shared::world::dimension::Dimension;
use std::collections::HashSet;
use std::ops::Range;
use std::path::Path;
use vek::{Vec2, Vec3};

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

    type ConstructionInformation;

    fn open(
        path: Box<Path>,
        create_if_missing: bool,
        information: &mut Self::ConstructionInformation,
    ) -> Result<Self, Self::Err>;

    fn close(&mut self) -> Result<(), Self::Err>;

    fn write_bytes_to_key(
        &mut self,
        chunk_info: ChunkKey,
        chunk_bytes: &[u8],
    ) -> Result<(), Self::Err>;

    fn get_bytes_from_key(&mut self, chunk_info: ChunkKey) -> Result<Option<Vec<u8>>, Self::Err>;

    fn delete_bytes_at_key(&mut self, chunk_info: ChunkKey) -> Result<(), Self::Err>;

    fn set_sub_chunk_raw(
        &mut self,
        chunk_info: ChunkKey,
        chunk_bytes: &[u8],
    ) -> Result<(), Self::Err> {
        self.write_bytes_to_key(chunk_info, chunk_bytes)
    }

    fn get_sub_chunk_raw(&mut self, chunk_info: ChunkKey) -> Result<Option<Vec<u8>>, Self::Err> {
        self.get_bytes_from_key(chunk_info)
    }

    fn chunk_exists(&mut self, chunk_info: ChunkKey) -> Result<bool, Self::Err> {
        Ok(self.get_bytes_from_key(chunk_info)?.is_some())
    }

    fn write_sub_chunk_batch(
        &mut self,
        sub_chunk_batch_info: Vec<(ChunkKey, Vec<u8>)>,
    ) -> Result<(), Self::Err>;

    fn write_sub_chunk_marker_batch(
        &mut self,
        sub_chunk_batch_info: Vec<ChunkKey>,
    ) -> Result<(), Self::Err>;

    fn mark_exist_chunk(&mut self, chunk_info: ChunkKey) -> Result<(), Self::Err> {
        self.write_bytes_to_key(chunk_info, &[])
    }

    fn build_key(key: &ChunkKey) -> Vec<u8>;

    fn generated_chunks(&mut self) -> Result<HashSet<(Dimension, Vec2<i32>)>, Self::Err>;

    fn delete_chunk(
        &mut self,
        xz: Vec2<i32>,
        dimension: Dimension,
        sub_chunk_range: Vec2<i8>,
    ) -> Result<(), Self::Err> {
        for y in sub_chunk_range.x..=sub_chunk_range.y {
            self.delete_sub_chunk((xz.x, y as i32, xz.y).into(), dimension)?
        }

        self.delete_bytes_at_key(ChunkKey::chunk_marker(xz, dimension))?;

        Ok(())
    }

    fn delete_sub_chunk(&mut self, xyz: Vec3<i32>, dimension: Dimension) -> Result<(), Self::Err> {
        self.delete_bytes_at_key(ChunkKey::new_sub_chunk(xyz, dimension))
    }
}
