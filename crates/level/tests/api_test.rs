use bedrockrs_level::level::level::{
    BedrockLevel, FillConfig, GetBlockConfig, LevelConfiguration, LevelDbError, SetBlockConfig,
};
use bedrockrs_level::level::sub_chunk::{SerDeStore, SubChunk, SubChunkSerDe};
use bedrockrs_level::level::world_block::LevelBlock;
use bedrockrs_shared::world::dimension::Dimension;

// Needed for the test only
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
        println!("{:?}", k);
    }
    println!("dump end");
}

#[cfg(feature = "default-impl")]
#[test]
fn world_test() -> Result<(), anyhow::Error> {
    println!("Loading World");

    let mut level = get_level_with_copy()?;

    let block = LevelBlock::id("minecraft:tnt".into());
    let range = 48;
    // level
    //     .fill(FillConfig {
    //         from: (-range, -range, -range).into(),
    //         to: (30, -30, 30).into(),
    //         block: block.clone(),
    //         dim: Dimension::Overworld,
    //         layer: 0,
    //         data_version: 9,
    //     })
    //     .expect("TODO: panic message");
    // level.flush()?;
    // drop(level);

    level
        .set_block(SetBlockConfig {
            position: (0, 10, 0).into(),
            block: block.clone(),
            dim: Dimension::Overworld,
            layer: 0,
        })
        .expect("Cant Fail");
    // level
    //     .set_block(SetBlockConfig {
    //         position: (0, -3, 0).into(),
    //         block: block.clone(),
    //         dim: Dimension::Overworld,
    //         layer: 0,
    //     })
    //     .expect("Cant Fail");

    // println!(
    //     "{:?}",
    //     level
    //         .get_block::<LevelBlock>(GetBlockConfig {
    //             position: (0, 13, 0).into(),
    //             dim: Dimension::Overworld,
    //             layer: 0,
    //         })
    //         .unwrap()
    // );
    // level.close()?;

    level.flush()?;
    dump_keys(&mut level);
    // let mut level = get_level_with_copy()?;
    //
    // println!(
    //     "{:?}",
    //     level
    //         .get_block::<LevelBlock>(GetBlockConfig {
    //             position: (0, 13, 0).into(),
    //             dim: Dimension::Overworld,
    //             layer: 0,
    //         })
    //         .unwrap()
    // );
    // println!(
    //     "{:?}",
    //     level
    //         .get_block::<LevelBlock>(GetBlockConfig {
    //             position: (0, -3, 0).into(),
    //             dim: Dimension::Overworld,
    //             layer: 0,
    //         })
    //         .unwrap()
    // );

    std::process::exit(0);
    let mut sub_chunk = level
        .get_sub_chunk::<SubChunk, SubChunkSerDe>(
            (0, 0, 0).into(),
            Dimension::Overworld,
            SerDeStore::default(),
        )
        .expect("Read failed")
        .unwrap_or_else(|| SubChunk::empty((0, 0, 0).into(), Dimension::Overworld).to_init());

    sub_chunk
        .set_block((0, 0, 0).into(), LevelBlock::id("minecraft:tnt".into()))
        .expect("");

    level
        .set_sub_chunk::<SubChunk, SubChunkSerDe>(&sub_chunk, SerDeStore::default())
        .expect("This should never fail");

    assert_eq!(
        level
            .get_block::<LevelBlock>(GetBlockConfig {
                position: (0, -3, 0).into(),
                dim: Dimension::Overworld,
                layer: 0,
            })
            .expect("Cant Fail")
            .unwrap(),
        block
    );

    level
        .fill(FillConfig {
            from: (-11, -11, -11).into(),
            to: (10, 10, 10).into(),
            block: block.clone(),
            dim: Dimension::Overworld,
            layer: 0,
            data_version: 9,
        })
        .expect("TODO: panic message");

    Ok(())
}
