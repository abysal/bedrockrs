use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SubChunkError {
    #[error("Failed To Get Layer: {0}")]
    LayerError(u8),
}

#[derive(Error, Debug)]
pub enum SubChunkSerDeError<WorldError: Debug, SerDeError: Debug> {
    #[error(transparent)]
    WorldError(WorldError),
    #[error(transparent)]
    SerDeError(SerDeError),
}

#[derive(Error, Debug)]
pub enum SubChunkReadWriteError<WorldError: Debug, SerDeError: Debug, TranslationError: Debug> {
    #[error(transparent)]
    TranslationError(TranslationError),
    #[error(transparent)]
    WorldError(WorldError),
    #[error(transparent)]
    SerDeError(SerDeError),
    #[error(transparent)]
    SubChunkError(#[from] SubChunkError),
}

#[derive(Error, Debug)]
pub enum LevelFilLError<WorldError: Debug, Decode: Debug, Encode: Debug, TranslationError: Debug> {
    #[error(transparent)]
    TranslationError(TranslationError),
    #[error(transparent)]
    WorldError(WorldError),
    #[error(transparent)]
    DecodeError(Decode),
    #[error(transparent)]
    EncoderError(Encode),
    #[error(transparent)]
    SubChunkError(#[from] SubChunkError),
}

impl<WorldError: Debug, Decode: Debug, Encode: Debug, TranslationError: Debug>
    LevelFilLError<WorldError, Decode, Encode, TranslationError>
{
    pub fn from_decode(
        other: SubChunkReadWriteError<WorldError, Decode, TranslationError>,
    ) -> Self {
        match other {
            SubChunkReadWriteError::TranslationError(r) => Self::TranslationError(r),
            SubChunkReadWriteError::WorldError(r) => Self::WorldError(r),
            SubChunkReadWriteError::SerDeError(r) => Self::DecodeError(r),
            SubChunkReadWriteError::SubChunkError(r) => Self::SubChunkError(r),
        }
    }

    pub fn from_encode(
        other: SubChunkReadWriteError<WorldError, Encode, TranslationError>,
    ) -> Self {
        match other {
            SubChunkReadWriteError::TranslationError(r) => Self::TranslationError(r),
            SubChunkReadWriteError::WorldError(r) => Self::WorldError(r),
            SubChunkReadWriteError::SerDeError(r) => Self::EncoderError(r),
            SubChunkReadWriteError::SubChunkError(r) => Self::SubChunkError(r),
        }
    }
}

impl<WorldError: Debug, SerDeError: Debug, TranslationError: Debug>
    From<SubChunkSerDeError<WorldError, SerDeError>>
    for SubChunkReadWriteError<WorldError, SerDeError, TranslationError>
{
    fn from(value: SubChunkSerDeError<WorldError, SerDeError>) -> Self {
        match value {
            SubChunkSerDeError::WorldError(e) => SubChunkReadWriteError::WorldError(e),
            SubChunkSerDeError::SerDeError(e) => SubChunkReadWriteError::SerDeError(e),
        }
    }
}
