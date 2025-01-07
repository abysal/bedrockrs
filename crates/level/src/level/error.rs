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
pub enum WholeLevelError<WorldError: Debug, SerDeError: Debug, TranslationError: Debug> {
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
pub enum WorldPipelineError<
    WorldError: Debug,
    Decode: Debug,
    Encode: Debug,
    TranslationError: Debug,
> {
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
    WorldPipelineError<WorldError, Decode, Encode, TranslationError>
{
    pub fn from_decode(other: WholeLevelError<WorldError, Decode, TranslationError>) -> Self {
        match other {
            WholeLevelError::TranslationError(r) => Self::TranslationError(r),
            WholeLevelError::WorldError(r) => Self::WorldError(r),
            WholeLevelError::SerDeError(r) => Self::DecodeError(r),
            WholeLevelError::SubChunkError(r) => Self::SubChunkError(r),
        }
    }

    pub fn from_encode(other: WholeLevelError<WorldError, Encode, TranslationError>) -> Self {
        match other {
            WholeLevelError::TranslationError(r) => Self::TranslationError(r),
            WholeLevelError::WorldError(r) => Self::WorldError(r),
            WholeLevelError::SerDeError(r) => Self::EncoderError(r),
            WholeLevelError::SubChunkError(r) => Self::SubChunkError(r),
        }
    }
}

impl<WorldError: Debug, SerDeError: Debug, TranslationError: Debug>
    From<SubChunkSerDeError<WorldError, SerDeError>>
    for WholeLevelError<WorldError, SerDeError, TranslationError>
{
    fn from(value: SubChunkSerDeError<WorldError, SerDeError>) -> Self {
        match value {
            SubChunkSerDeError::WorldError(e) => WholeLevelError::WorldError(e),
            SubChunkSerDeError::SerDeError(e) => WholeLevelError::SerDeError(e),
        }
    }
}
