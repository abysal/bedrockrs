pub use crate::level::error::SubChunkError;
use crate::level::world_block::{BlockTransition, LevelBlock};
use crate::utility::miner::idx_3_to_1;
use bedrockrs_shared::world::dimension::Dimension;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use nbtx::NbtError;
use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use std::io::Write;
use std::mem::MaybeUninit;
use thiserror::Error;
use vek::{Vec2, Vec3};

pub type BlockLayer<T> = (Box<[u16; 4096]>, Vec<T>);

#[allow(dead_code)]
pub struct SubChunkTransition {
    position: Vec3<i32>,
    data_version: u8,
    layers: Vec<BlockLayer<LevelBlock>>,
}
impl Debug for SubChunkTransition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SubChunkTransition(position ({}), data_version ({}), layer count ({}), ",
            self.position,
            self.data_version,
            self.layers.len()
        )?;
        for l in &self.layers {
            write!(f, "Layer block count ({})", l.1.len())?;
        }

        write!(f, ")")
    }
}

impl SubChunkTransition {
    pub fn new(position: Vec3<i32>, layer_count: usize, version: u8) -> Self {
        Self {
            position,
            data_version: version,
            layers: Vec::with_capacity(layer_count),
        }
    }

    pub fn new_layer(&mut self, data: (Box<[u16; 4096]>, Vec<LevelBlock>)) {
        self.layers.push(data);
    }

    pub fn full(position: Vec3<i32>, version: u8, block: LevelBlock) -> Self {
        let mut ret = Self::new(position, 1, version);
        let buffer: BlockLayer<LevelBlock> = (Box::new([0u16; 4096]), vec![block]);
        ret.layers.push(buffer);
        ret
    }

    pub fn air(position: Vec3<i32>, version: u8) -> Self {
        Self::full(position, version, LevelBlock::air())
    }
}

pub trait SubChunkDecoder {
    type Err;

    /// This function is responsible for decoding a stream of raw bytes into the intermediate structure used for creating sub chunks
    fn decode_bytes_as_sub_chunk(
        &mut self,
        bytes: &mut Cursor<Vec<u8>>,
        xz: Vec2<i32>,
    ) -> Result<SubChunkTransition, Self::Err>;
}

pub trait SubChunkEncoder {
    type Err;

    /// This function is responsible
    /// for encoding the raw block data into a vector of bytes which the database layer can then save.
    fn write_as_bytes(
        &mut self,
        chunk_state: SubChunkTransition,
        network: bool,
    ) -> Result<Vec<u8>, Self::Err>;
}

pub struct SerDeStore<SerDeType, TransitionType = ()> {
    pub serde: SerDeType,
    pub information: TransitionType,
}

pub struct SerDeStoreRef<'a, 'u, SerDeType, TransitionType> {
    pub serde: &'a mut SerDeType,
    pub information: &'u mut TransitionType,
}

impl<'a, 'u, SerDeType, TransitionType> SerDeStoreRef<'a, 'u, SerDeType, TransitionType> {
    pub fn new(serde: &'a mut SerDeType, information: &'u mut TransitionType) -> Self {
        Self { serde, information }
    }
}

pub trait SerDeTrait {
    type SerDe;
    type Info;
    fn serde(&mut self) -> &mut Self::SerDe;
    fn info(&mut self) -> &mut Self::Info;
}

impl<SerDeType, TransitionType> SerDeTrait for SerDeStoreRef<'_, '_, SerDeType, TransitionType> {
    type SerDe = SerDeType;
    type Info = TransitionType;

    fn serde(&mut self) -> &mut Self::SerDe {
        self.serde
    }

    fn info(&mut self) -> &mut Self::Info {
        self.information
    }
}

impl<SerDeType, TransitionType> SerDeTrait for SerDeStore<SerDeType, TransitionType> {
    type SerDe = SerDeType;
    type Info = TransitionType;

    fn serde(&mut self) -> &mut Self::SerDe {
        &mut self.serde
    }

    fn info(&mut self) -> &mut Self::Info {
        &mut self.information
    }
}

impl<SerDeType: Default, Transition: Default> Default for SerDeStore<SerDeType, Transition> {
    fn default() -> Self {
        Self {
            serde: SerDeType::default(),
            information: Transition::default(),
        }
    }
}

/// The main trait that any sub chunk type must implement
pub trait SubChunkTrait: Sized {
    type TransitionInformation;
    type Err;
    /// This must create a valid "full" state for the sub chunk from the transitional data
    fn decode_from_transition(
        data: SubChunkTransition,
        dimension: Dimension,
        info: &mut Self::TransitionInformation,
    ) -> Result<Self, Self::Err>;

    /// This must create a transitional state from the current sub chunk information
    fn to_transition(
        &self,
        info: &mut Self::TransitionInformation,
    ) -> Result<SubChunkTransition, Self::Err>;

    /// This returns if the sub chunk is just air if so nothing is written to the database if this isn't desired behavior just always return false
    fn is_empty(&self) -> bool;
}

/// An extension of the SubChunkTrait which must be implemented if the simple functions want to be called. Such as [`Level::write_sub_chunk`]
pub trait SubChunkTraitExtended: SubChunkTrait {
    fn dimension(&self) -> Dimension;
    fn position(&self) -> Vec3<i32>;
}

#[derive(Default, Debug)]
pub struct SubChunkSerDe;

#[derive(Debug, Error)]
pub enum SubChunkDecoderError {
    #[error("Missing Subchunk Version")]
    SubChunkVersion,
    #[error("Unknown Subchunk Version: {0}")]
    UnknownVersion(u8),
    #[error("Failed To Read Layer Count")]
    LayerError,
    #[error("Failed To Read Y Index")]
    IndexError,
    #[error("Failed To Read Palette Type")]
    PaletteError,
    #[error("Failed To Read Index Word")]
    WordError,
    #[error("Failed To Read Palette Count")]
    PaletteCountError,
    #[error("Failed To Slice NBT")]
    SliceError,
    #[error("Binary Error: {0}")]
    BinaryError(#[from] std::io::Error),
    #[error("NBT Error: {0}")]
    NBTError(#[from] NbtError),
}

impl SubChunkDecoder for SubChunkSerDe {
    type Err = SubChunkDecoderError;

    fn decode_bytes_as_sub_chunk(
        &mut self,
        bytes: &mut Cursor<Vec<u8>>,
        xz: Vec2<i32>,
    ) -> Result<SubChunkTransition, Self::Err> {
        let version = bytes.read_u8()?;
        if version != 8 && version != 9 {
            return Err(SubChunkDecoderError::UnknownVersion(version));
        }

        let storage_layer_count = bytes.read_u8()?;
        let y_index = bytes.read_i8()?;

        let mut transitiondata = SubChunkTransition::new(
            Vec3::new(xz.x, y_index as i32, xz.y),
            storage_layer_count as usize,
            version,
        );

        for _ in 0..storage_layer_count {
            let palette_type = bytes.read_u8()?;
            let network = palette_type & 0x1 == 1;
            let bits_per_block = palette_type >> 1;
            let blocks_per_word = 32 / bits_per_block;
            let word_count = (4096 + (blocks_per_word as i32) - 1) / (blocks_per_word as i32);
            let mask = (1 << bits_per_block) - 1;
            let mut pos = 0usize;
            let mut block_indices = Box::new([0u16; 4096]);

            for _ in 0..word_count {
                let mut word = bytes.read_u32::<LittleEndian>()?;
                for _ in 0..blocks_per_word {
                    let index: u16 = (word & mask) as u16;
                    if pos == 4096 {
                        break;
                    }
                    block_indices[pos] = index;
                    word >>= bits_per_block;
                    pos += 1;
                }
            }

            let palette_count = bytes.read_u32::<LittleEndian>()?;
            let mut blocks = Vec::with_capacity(palette_count as usize);
            for _ in 0_usize..palette_count as usize {
                if network {
                    blocks.push(LevelBlock::from_transition(nbtx::from_bytes::<
                        nbtx::NetworkLittleEndian,
                        LevelBlock,
                    >(bytes)?));
                } else {
                    blocks.push(LevelBlock::from_transition(nbtx::from_bytes::<
                        nbtx::LittleEndian,
                        LevelBlock,
                    >(bytes)?));
                }
            }
            transitiondata.new_layer((block_indices, blocks));
        }
        Ok(transitiondata)
    }
}

impl SubChunkEncoder for SubChunkSerDe {
    type Err = SubChunkDecoderError;

    // TODO: Handle 0, 2, 3, 4 ,5 ,6 7, also handle 1
    fn write_as_bytes(
        &mut self,
        chunk_state: SubChunkTransition,
        network: bool,
    ) -> Result<Vec<u8>, Self::Err> {
        let mut buffer = Cursor::<Vec<u8>>::new(vec![]);
        buffer.write_u8(chunk_state.data_version)?;
        buffer.write_u8(chunk_state.layers.len() as u8)?;
        buffer.write_i8(chunk_state.position.y as i8)?;
        for layer in chunk_state.layers {
            let bits_per_block = bits_needed_to_store(layer.1.len() as u32);
            buffer.write_u8(bits_per_block << (1 + (network as u8)))?;

            let mut current_word = 0u32;
            let mut bits_written = 0;
            layer.0.iter().try_for_each(|element| {
                let element = *element as u32;
                if bits_written + bits_per_block > 32 {
                    buffer.write_u32::<LittleEndian>(current_word)?;
                    current_word = 0;
                    bits_written = 0;
                }

                current_word = current_word + (element << bits_written);
                bits_written += bits_per_block;
                Ok::<(), std::io::Error>(())
            })?;
            if bits_written != 0 {
                buffer.write_u32::<LittleEndian>(current_word)?;
            }
            buffer.write_u32::<LittleEndian>(layer.1.len() as u32)?;
            for blk in layer.1 {
                if network {
                    buffer.write(&nbtx::to_net_bytes(&blk.into_transition())?)?
                } else {
                    buffer.write(&nbtx::to_le_bytes(&blk.into_transition())?)?
                };
            }
        }
        Ok(buffer.into_inner())
    }
}

#[derive(Clone, PartialEq)]
pub struct SubChunk {
    blocks: Vec<Box<[LevelBlock; 4096]>>,
    position: Vec3<i32>,
    dimension: Dimension,
    active_layer: u8,
    is_empty: bool,
}

impl Debug for SubChunk {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SubChunk(Blocks: (..., {} elements), position: {}, active_layer: {}, is_empty: {})",
            self.blocks.len(),
            self.position,
            self.active_layer,
            self.is_empty
        )
    }
}

impl SubChunk {
    pub fn force_non_empty(&mut self) {
        self.is_empty = false;
    }

    pub fn force_empty(&mut self) {
        self.is_empty = true;
    }

    pub fn full(position: Vec3<i32>, dimension: Dimension, block: impl BlockTransition) -> Self {
        let mut val = Self {
            blocks: Vec::with_capacity(1),
            position,
            dimension,
            active_layer: 0,
            is_empty: false,
        };
        val.blocks.push(Box::new(std::array::from_fn(|_| {
            block.clone().into_transition()
        })));
        val
    }

    pub fn empty(position: Vec3<i32>, dimension: Dimension) -> Self {
        let mut val = Self {
            blocks: Vec::with_capacity(1),
            position,
            dimension,
            active_layer: 0,
            is_empty: true,
        };
        val.blocks
            .push(Box::new(std::array::from_fn(|_| LevelBlock::air())));
        val
    }

    pub fn to_init(mut self) -> Self {
        self.is_empty = false;
        self
    }

    pub fn get_block(&self, xyz: Vec3<u8>) -> Option<&LevelBlock> {
        let layer = self.blocks.get(self.active_layer as usize)?;
        layer.get(idx_3_to_1::<u8>(xyz, 16u8, 16u8))
    }

    pub fn get_block_mut(&mut self, xyz: Vec3<u8>) -> Option<&mut LevelBlock> {
        let layer = self.blocks.get_mut(self.active_layer as usize)?;
        layer.get_mut(idx_3_to_1::<u8>(xyz, 16u8, 16u8))
    }

    pub fn set_block<Block: BlockTransition>(
        &mut self,
        xyz: Vec3<u8>,
        block: Block,
    ) -> Result<(), SubChunkError> {
        let layer = self
            .blocks
            .get_mut(self.active_layer as usize)
            .ok_or(SubChunkError::LayerError(self.active_layer))?;
        layer[idx_3_to_1::<u8>(xyz, 16u8, 16u8)] = block.into_transition();
        self.is_empty = false;
        Ok(())
    }

    pub fn get_active_layer(&self) -> u8 {
        self.active_layer
    }

    pub fn set_active_layer(&mut self, idx: u8) {
        if idx as usize >= self.blocks.len() {
            panic!(
                "Selected sub chunk index outside of valid range!, Layer Count: {}",
                self.blocks.len()
            )
        }
        self.active_layer = idx;
    }

    pub fn add_sub_layer(&mut self) {
        self.blocks
            .push(Box::new(std::array::from_fn(|_| LevelBlock::air())));
    }

    pub fn get_sub_layer_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn encode_single_layer(&self, layer_override: usize) -> BlockLayer<LevelBlock> {
        let mut indices = Box::new([0u16; 4096]);
        let mut unique_block_array = Vec::new();
        let layer = &self.blocks[layer_override];
        for z in 0..16u8 {
            for y in 0..16u8 {
                for x in 0..16u8 {
                    let current_block = &layer[idx_3_to_1((x, y, z).into(), 16, 16)];
                    if let Some(index) = unique_block_array
                        .iter()
                        .position(|ele| ele == current_block)
                    {
                        indices[idx_3_to_1((x, y, z).into(), 16, 16)] = index as u16;
                    } else {
                        unique_block_array.push(current_block.clone());
                        indices[idx_3_to_1((x, y, z).into(), 16, 16)] =
                            (unique_block_array.len() - 1) as u16;
                    }
                }
            }
        }
        (indices, unique_block_array)
    }
}
impl SubChunkTraitExtended for SubChunk {
    fn dimension(&self) -> Dimension {
        self.dimension
    }

    fn position(&self) -> Vec3<i32> {
        self.position
    }
}

impl SubChunkTrait for SubChunk {
    type TransitionInformation = ();
    type Err = ();

    fn decode_from_transition(
        data: SubChunkTransition,
        dimension: Dimension,
        _: &mut Self::TransitionInformation,
    ) -> Result<Self, Self::Err> {
        let mut layers: Vec<Box<[MaybeUninit<LevelBlock>; 4096]>> = (0..data.layers.len())
            .map(|_| Box::new([const { MaybeUninit::uninit() }; 4096]))
            .collect();
        for (layer_index, (indices, blocks)) in data.layers.into_iter().enumerate() {
            let layer: &mut Box<[MaybeUninit<LevelBlock>; 4096]> = &mut layers[layer_index];
            for whole_index in 0..4096usize {
                layer[whole_index].write(LevelBlock::from_other(
                    &blocks[indices[whole_index] as usize],
                ));
            }
        }

        let layers = unsafe { std::mem::transmute(layers) };

        Ok(Self {
            blocks: layers,
            position: data.position,
            dimension,
            active_layer: 0,
            is_empty: false,
        })
    }

    fn to_transition(
        &self,
        _: &mut Self::TransitionInformation,
    ) -> Result<SubChunkTransition, Self::Err> {
        let mut layers: Vec<BlockLayer<LevelBlock>> = Vec::with_capacity(self.blocks.len());
        for layer in 0..self.blocks.len() {
            layers.push(self.encode_single_layer(layer));
        }
        Ok(SubChunkTransition {
            layers,
            data_version: 9, // TODO: Change this to be configurable
            position: self.position,
        })
    }

    fn is_empty(&self) -> bool {
        self.is_empty
    }
}

fn bits_needed_to_store(val: u32) -> u8 {
    if val == 0 {
        1
    } else {
        (32 - val.leading_zeros()) as u8
    }
}
