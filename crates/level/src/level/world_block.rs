use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vek::{Vec2, Vec3};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum BlockStateValue {
    String(String),
    Bool(bool),
    Int(i32),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct LevelBlock {
    // If the name of anything in this struct changes the world ends
    pub name: String,
    pub states: HashMap<String, BlockStateValue>,
}

pub trait BlockTransition: Clone + PartialEq<Self> {
    fn from_transition(value: LevelBlock) -> Self;
    fn into_transition(self) -> LevelBlock;

    fn get_id(&self) -> &str;
    fn from_other(other: &Self) -> Self {
        other.clone()
    }
}

impl LevelBlock {
    pub fn air() -> Self {
        Self {
            name: String::from("minecraft:air"),
            states: HashMap::new(),
        }
    }

    pub fn id(id: String) -> Self {
        Self {
            name: id,
            states: HashMap::new(),
        }
    }

    pub fn block_pos_to_sub_chunk(pos: Vec3<i32>) -> (Vec3<i32>, Vec3<u8>) {
        let sub_chunk_y = (pos.y as f32 / 16f32).floor() as i8;
        let sub_chunk_y_inside = Self::interior_position(pos.y);

        let chunk_xz = Vec2::new(
            (pos.x as f32 / 16f32).floor() as i32,
            (pos.z as f32 / 16f32).floor() as i32,
        );

        let sub_chunk_xz = Vec2::new(
            Self::interior_position(pos.x),
            Self::interior_position(pos.z),
        );

        (
            (chunk_xz.x, sub_chunk_y as i32, chunk_xz.y).into(),
            (sub_chunk_xz.x, sub_chunk_y_inside, sub_chunk_xz.y).into(),
        )
    }

    fn interior_position(pos: i32) -> u8 {
        if pos >= 0 {
            (pos % 16) as u8
        } else {
            (pos % 16 + 16) as u8
        }
    }
}

impl<IdType: Into<String>, StateType: Into<HashMap<String, BlockStateValue>>>
    From<(IdType, StateType)> for LevelBlock
{
    fn from((id, states): (IdType, StateType)) -> Self {
        Self {
            name: id.into(),
            states: states.into(),
        }
    }
}

impl BlockTransition for LevelBlock {
    fn from_transition(value: LevelBlock) -> Self {
        value
    }

    fn into_transition(self) -> LevelBlock {
        self
    }

    fn get_id(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn position_translation_tests() {
        // Test values stolen from minecraft java's debug menu

        assert_eq!(
            LevelBlock::block_pos_to_sub_chunk((117, 73, -163).into()),
            ((7, 4, -11).into(), (5, 9, 13).into())
        );
        assert_eq!(
            LevelBlock::block_pos_to_sub_chunk((117, -53, -163).into()),
            ((7, -4, -11).into(), (5, 11, 13).into())
        );
        assert_eq!(
            LevelBlock::block_pos_to_sub_chunk((164, -39, -219).into()),
            ((10, -3, -14).into(), (4, 9, 5).into())
        );
        assert_eq!(
            LevelBlock::block_pos_to_sub_chunk((0, -3, 0).into()),
            ((0, -1, 0).into(), (0, 13, 0).into())
        );
    }
}
