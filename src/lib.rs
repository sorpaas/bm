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

pub use crate::traits::{MerkleDB, InMemoryMerkleDB, Value, ValueOf, IntermediateOf, IntermediateSizeOf, EndOf, DanglingRoot, OwnedRoot, RootStatus, Error};
pub use crate::raw::{MerkleRaw, OwnedMerkleRaw, DanglingMerkleRaw};
pub use crate::index::{MerkleIndex, MerkleSelection, MerkleRoute};
pub use crate::empty::{MerkleEmpty, OwnedMerkleEmpty, DanglingMerkleEmpty};
pub use crate::tuple::{MerkleTuple, OwnedMerkleTuple, DanglingMerkleTuple};
pub use crate::vec::{MerkleVec, OwnedMerkleVec, DanglingMerkleVec};
pub use crate::packed::{MerklePackedTuple, OwnedMerklePackedTuple, DanglingMerklePackedTuple,
                        MerklePackedVec, OwnedMerklePackedVec, DanglingMerklePackedVec};
