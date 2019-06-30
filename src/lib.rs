#![warn(missing_docs)]

//! Binary merkle tree implementation.

mod traits;
mod raw;
mod index;
mod tuple;
mod vec;
mod packed;
mod proving;

pub use crate::traits::{MerkleDB, InMemoryMerkleDB, Value, ValueOf, IntermediateOf, EndOf, DanglingRoot, OwnedRoot, RootStatus, Error};
pub use crate::raw::{MerkleRaw, OwnedMerkleRaw, DanglingMerkleRaw};
pub use crate::index::{MerkleIndex, MerkleSelection, MerkleRoute};
pub use crate::tuple::{MerkleTuple, OwnedMerkleTuple, DanglingMerkleTuple};
pub use crate::vec::{MerkleVec, OwnedMerkleVec, DanglingMerkleVec};
pub use crate::packed::{MerklePackedTuple, OwnedMerklePackedTuple, DanglingMerklePackedTuple,
                        MerklePackedVec, OwnedMerklePackedVec, DanglingMerklePackedVec};
pub use crate::proving::ProvingMerkleDB;
