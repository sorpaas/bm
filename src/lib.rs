#![warn(missing_docs)]

//! Binary merkle tree implementation.

mod traits;
mod raw;
mod index;
mod utils;
mod empty;
mod tuple;
mod vec;
mod packed;

pub use crate::traits::{MerkleDB, InMemoryMerkleDB, Value, ValueOf, IntermediateOf, IntermediateSizeOf, EndOf, DanglingRoot, OwnedRoot, RootStatus};
pub use crate::raw::MerkleRaw;
pub use crate::index::{MerkleIndex, MerkleSelection, MerkleRoute};
pub use crate::empty::MerkleEmpty;
pub use crate::tuple::MerkleTuple;
pub use crate::vec::MerkleVec;
pub use crate::packed::{MerklePackedTuple, MerklePackedVec};
