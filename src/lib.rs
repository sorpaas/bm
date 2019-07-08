#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

//! Binary merkle tree implementation.

extern crate alloc;

mod traits;
mod memory;
mod raw;
mod index;
mod vector;
mod list;
mod packed;
mod length;
#[cfg(feature = "std")]
mod proving;

pub mod utils;

pub use crate::traits::{Backend, Value, ValueOf, IntermediateOf, EndOf, Dangling, Owned, RootStatus, Error, Sequence, Tree, Leak};
pub use crate::memory::{InMemoryBackend, InMemoryBackendError, NoopBackend, NoopBackendError};
pub use crate::raw::{Raw, OwnedRaw, DanglingRaw};
pub use crate::index::{Index, IndexSelection, IndexRoute};
pub use crate::vector::{Vector, OwnedVector, DanglingVector};
pub use crate::list::{List, OwnedList, DanglingList};
pub use crate::packed::{PackedVector, OwnedPackedVector, DanglingPackedVector,
                        PackedList, OwnedPackedList, DanglingPackedList};
pub use crate::length::LengthMixed;
#[cfg(feature = "std")]
pub use crate::proving::ProvingBackend;
