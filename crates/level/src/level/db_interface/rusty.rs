use crate::level::db_interface::bedrock_key::ChunkKey;
use crate::level::db_interface::db::LevelDBKey;
use crate::level::db_interface::key_level::KeyTypeTag;
use crate::level::file_interface::RawWorldTrait;
use bedrockrs_shared::world::dimension::Dimension;
use byteorder::{LittleEndian, ReadBytesExt};
use miniz_oxide::deflate::{compress_to_vec, compress_to_vec_zlib, CompressionLevel};
use miniz_oxide::inflate::{decompress_to_vec, decompress_to_vec_zlib};
use rusty_leveldb::compressor::NoneCompressor;
use rusty_leveldb::{Compressor, CompressorList, LdbIterator, Options, Status, WriteBatch, DB};
use std::collections::HashSet;
use std::io::Cursor;
use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;
use thiserror::Error;
use vek::Vec2;

struct ZlibCompressor(u8);

impl ZlibCompressor {
    pub fn new(level: u8) -> Self {
        assert!(level <= 10);
        Self(level)
    }
}

impl Compressor for ZlibCompressor {
    fn encode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        Ok(compress_to_vec_zlib(&block, self.0))
    }

    fn decode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        decompress_to_vec_zlib(&block).map_err(|e| rusty_leveldb::Status {
            code: rusty_leveldb::StatusCode::CompressionError,
            err: e.to_string(),
        })
    }
}

struct RawZlibCompressor(u8);

impl RawZlibCompressor {
    pub fn new(level: u8) -> Self {
        assert!(level <= 10);
        Self(level)
    }
}
impl Compressor for RawZlibCompressor {
    fn encode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        Ok(compress_to_vec(&block, self.0))
    }

    fn decode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        decompress_to_vec(&block).map_err(|e| rusty_leveldb::Status {
            code: rusty_leveldb::StatusCode::CompressionError,
            err: e.to_string(),
        })
    }
}

pub fn mcpe_options(compression_level: u8) -> Options {
    let mut opt = Options::default();
    let mut list = CompressorList::new();
    list.set_with_id(0, NoneCompressor {});
    list.set_with_id(2, ZlibCompressor::new(compression_level));
    list.set_with_id(4, RawZlibCompressor::new(compression_level));
    opt.compressor_list = Rc::new(list);
    opt.compressor = 4;
    opt
}
const COMPRESSION_LEVEL: u8 = CompressionLevel::DefaultLevel as u8;

pub struct RustyDBInterface<UserState> {
    db: DB,
    phantom_data: PhantomData<UserState>,
}

#[derive(Debug, Error)]
pub enum DBError {
    #[error("Rusty DB: {0}")]
    DatabaseError(#[from] Status),
}

impl<UserState> RustyDBInterface<UserState> {
    fn build_key_batch(subchunk_batch_info: Vec<ChunkKey>, data: &mut Vec<u8>) -> WriteBatch {
        let count = subchunk_batch_info
            .iter()
            .map(|ele| ele.estimate_size())
            .sum();
        data.resize(count, 0);

        let mut buff: Cursor<&mut [u8]> = Cursor::new(data);

        let mut batch = WriteBatch::default();

        for key in subchunk_batch_info {
            let start = buff.position();

            key.write_key(&mut buff);

            let end = buff.position();

            let whole = buff.get_ref();

            batch.put(&whole[start as usize..end as usize], &[]);
        }
        batch
    }

    pub fn get_key(&mut self, chunk_info: ChunkKey) -> Option<Vec<u8>> {
        self.db.get(&Self::build_key(&chunk_info))
    }
}

impl<T> Drop for RustyDBInterface<T> {
    fn drop(&mut self) {
        self.db.close().unwrap();
    }
}

impl<UserState> RawWorldTrait for RustyDBInterface<UserState> {
    type Err = DBError;
    type UserState = UserState;

    fn write_bytes_to_key(
        &mut self,
        chunk_info: ChunkKey,
        chunk_bytes: &[u8],
        _: &mut Self::UserState,
    ) -> Result<(), Self::Err> {
        let mut batch = WriteBatch::default();
        batch.put(&Self::build_key(&chunk_info), chunk_bytes);
        Ok(self.db.write(batch, false)?)
    }

    fn get_bytes_from_key(
        &mut self,
        chunk_info: ChunkKey,
        _: &mut Self::UserState,
    ) -> Result<Option<Vec<u8>>, Self::Err> {
        Ok(self.db.get(&Self::build_key(&chunk_info)))
    }

    fn delete_bytes_at_key(
        &mut self,
        chunk_info: ChunkKey,
        _: &mut Self::UserState,
    ) -> Result<(), Self::Err> {
        Ok(self.db.delete(&Self::build_key(&chunk_info))?)
    }

    fn write_subchunk_batch(
        &mut self,
        subchunk_batch_info: Vec<(ChunkKey, Vec<u8>)>,
        _: &mut Self::UserState,
    ) -> Result<(), Self::Err> {
        let mut data: Vec<u8> = vec![
            0;
            subchunk_batch_info
                .iter()
                .map(|(info, _)| info.estimate_size())
                .sum()
        ];
        let mut buff: Cursor<&mut [u8]> = Cursor::new(&mut data);
        let mut batch = WriteBatch::default();
        for (key, _) in &subchunk_batch_info {
            let start = buff.position();

            key.write_key(&mut buff);

            let end = buff.position();

            let whole = buff.get_ref();

            batch.put(&whole[start as usize..end as usize], &[]);
        }
        Ok(self.db.write(batch, false)?)
    }

    fn write_subchunk_marker_batch(
        &mut self,
        subchunk_batch_info: Vec<ChunkKey>,
        _: &mut Self::UserState,
    ) -> Result<(), Self::Err> {
        let mut data: Vec<u8> = Vec::new();
        let batch = Self::build_key_batch(subchunk_batch_info, &mut data);
        Ok(self.db.write(batch, false)?)
    }

    fn build_key(key: &ChunkKey) -> Vec<u8> {
        let mut key_bytes: Vec<u8> = vec![0; key.estimate_size()];
        let mut buff: Cursor<&mut [u8]> = Cursor::new(&mut key_bytes);
        key.write_key(&mut buff);
        key_bytes
    }

    fn new(
        path: Box<Path>,
        create_if_missing: bool,
        _: &mut Self::UserState,
    ) -> Result<Self, Self::Err> {
        let mut opts = mcpe_options(COMPRESSION_LEVEL);
        opts.create_if_missing = create_if_missing;
        let db = DB::open(path, opts)?;
        Ok(Self {
            db,
            phantom_data: PhantomData,
        })
    }

    fn close(&mut self) -> Result<(), Self::Err> {
        self.db.close()?;
        Ok(())
    }

    fn generated_chunks(
        &mut self,
        _: &mut Self::UserState,
    ) -> Result<HashSet<(Dimension, Vec2<i32>)>, Self::Err> {
        let mut out_set = HashSet::new();

        let mut iter = self.db.new_iter()?;

        loop {
            let mut key = Vec::new();
            let data = &mut Vec::new();

            iter.current(&mut key, data);
            let len = key.len();
            let mut cursor = Cursor::new(&mut key);

            if len == 9 || len == 13 {
                // Does a little hack to make sure it isn't reading a key that it doesn't want to
                if cursor.get_ref().get(len - 1) == Some(&KeyTypeTag::Version.to_byte()) {
                    let x = cursor
                        .read_i32::<LittleEndian>()
                        .expect("This should never fail");

                    let y = cursor
                        .read_i32::<LittleEndian>()
                        .expect("This should never fail");

                    let dim = if len == 13 {
                        cursor
                            .read_i32::<LittleEndian>()
                            .expect("This should never fail")
                            .into()
                    } else {
                        Dimension::Overworld
                    };
                    out_set.insert((dim, (x, y).into()));
                }
            }
            if !iter.advance() {
                break;
            }
        }
        Ok(out_set)
    }
}
