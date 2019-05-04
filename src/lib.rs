#![warn(missing_docs)]

//! Binary merkle tree implementation.

mod empty;
mod packed;
mod raw;
mod traits;
mod tuple;
mod utils;
mod vec;

pub use crate::empty::MerkleEmpty;
pub use crate::packed::{MerklePackedTuple, MerklePackedVec};
pub use crate::raw::MerkleRaw;
pub use crate::traits::{
    EndOf, InMemoryMerkleDB, IntermediateOf, IntermediateSizeOf, MerkleDB, Value, ValueOf,
};
pub use crate::tuple::MerkleTuple;
pub use crate::vec::MerkleVec;
