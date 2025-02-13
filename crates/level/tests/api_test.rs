use bedrockrs_level::level::level::{
    BedrockLevel, ChunkSelectionFilter, FillConfig, GetBlockConfig, LevelConfiguration,
    LevelDbError, SetBlockConfig,
};
use bedrockrs_level::level::sub_chunk::{
    SerDeStore, SubChunk, SubChunkSerDe, SubChunkTraitExtended,
};
use bedrockrs_level::level::world_block::LevelBlock;
use bedrockrs_shared::world::dimension::Dimension;

// Needed for the test only
use bedrockrs_level::level::file_interface::RawWorldTrait;
use bedrockrs_shared::world::dimension::Dimension::Overworld;
use copy_dir::copy_dir;
use rusty_leveldb::LdbIterator;
use std::path::Path;

#[cfg(feature = "default-impl")]
fn get_level_with_copy() -> Result<BedrockLevel, LevelDbError> {
    let _ = std::fs::remove_dir_all("./test_level_temp"); // If this throws an error its fine
    copy_dir("./test_level", "./test_level_temp").unwrap();
    BedrockLevel::open(
        Box::from(Path::new("./test_level_temp")),
        LevelConfiguration::default(),
        &mut (),
    )
}

fn dump_keys(level: &mut BedrockLevel) {
    level.underlying_world_interface().db.flush().unwrap();
    let mut iter = level.underlying_world_interface().db.new_iter().unwrap();
    iter.seek_to_first();

    println!("dump start");
    while let Some((k, v)) = iter.next() {
        println!("{:?} {}", k, v.len());
    }
    println!("dump end");
}

#[cfg(feature = "default-impl")]
#[test]
fn world_test() -> Result<(), anyhow::Error> {
    println!("Loading World");

    let mut level = get_level_with_copy()?;

    let exists = level.chunk_exists((0, 0).into(), Overworld);

    if !exists {
        panic!("Expected chunk doesnt exist!")
    }

    Ok(())
}
