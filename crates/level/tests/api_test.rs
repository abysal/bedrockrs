use bedrockrs_level::level::chunk::{FillFilter, LevelChunkTrait};
use bedrockrs_level::level::level::default_impl::*;
use bedrockrs_level::level::level::{ChunkSelectionFilter, LevelConfiguration};
use bedrockrs_level::level::sub_chunk::SubchunkFillFilter;
use bedrockrs_shared::world::dimension::Dimension;
use std::path::Path;

#[cfg(feature = "default-impl")]
#[test]
fn world_test(
) -> Result<(), BedrockLevelError<RawInterface, BedrockSubChunkDecoder, BedrockSubChunk>> {
    let wld_path = "./test_level";

    println!("Loading World");

    let mut level = BedrockLevel::open(
        Box::from(Path::new(wld_path)),
        LevelConfiguration::default(),
        BedrockState {},
    )?;

    println!("Collecting Chunks");
    let chunks = level.get_chunk_keys(ChunkSelectionFilter::Dimension(Dimension::Overworld));

    println!("Collected {} Chunks!", chunks.len());

    let blks = [
        BedrockWorldBlock::new("minecraft:iron_block".to_string()),
        BedrockWorldBlock::new("minecraft:diamond_block".to_string()),
    ];
    let len = chunks.len();

    println!("Filling Chunks");
    for (idx, key) in chunks.into_iter().enumerate() {
        let mut chunk = BedrockChunk::empty(
            key,
            (-4, 20).into(),
            Dimension::Overworld,
            &mut BedrockState {},
        );

        for blk in &blks {
            chunk
                .fill_chunk(
                    blk.clone(),
                    FillFilter::Precedence(Box::new(|_, _, _, _| rand::random::<f32>() > 0.5)),
                )
                .unwrap();
        }

        chunk.write_to_world(&mut level, None, None).unwrap();

        if idx % (len / 10 + 1) == 0 {
            println!("Wrote {idx} out of {len} chunks!");
        }
    }
    let mut chunk = level.get_chunk::<BedrockChunk>((0, 0).into(), Dimension::Overworld, None)?;

    chunk
        .fill_chunk(
            BedrockWorldBlock::new("minecraft:diamond_block".into()),
            FillFilter::Precedence(Box::new(|_, _, _, y| y == 0)),
        )
        .unwrap();
    level.set_chunk(chunk)?;

    let mut chunk = level.get_chunk::<BedrockChunk>((0, -1).into(), Dimension::Overworld, None)?;

    let subchunk = chunk.get_subchunk_mut(0).unwrap();
    subchunk
        .fill(
            &BedrockWorldBlock::new("minecraft:diamond_block".into()),
            SubchunkFillFilter::Blanket,
        )
        .unwrap();

    level.set_chunk(chunk)?;

    level.close()?;

    Ok(())
}
