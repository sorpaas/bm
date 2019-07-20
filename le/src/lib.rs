#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

//! SimpleSerialize (ssz) compliant binary merkle tree supporting both
//! merkleization and de-merkleization.

extern crate alloc;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use typenum::U32;
use generic_array::GenericArray;
use primitive_types::H256;
use digest::Digest;

pub use bm::{Backend, ReadBackend, WriteBackend, InheritedDigestConstruct,
             UnitDigestConstruct, Construct, InheritedEmpty, Error, ValueOf, Value, Vector,
             DanglingVector, List, Leak, NoopBackend, InMemoryBackend};

mod basic;
mod elemental_fixed;
mod elemental_variable;
mod fixed;
mod variable;
pub mod utils;

pub use elemental_fixed::{ElementalFixedVec, ElementalFixedVecRef,
                          IntoCompactVectorTree, FromCompactVectorTree,
                          IntoCompositeVectorTree, FromCompositeVectorTree};
pub use elemental_variable::{ElementalVariableVec, ElementalVariableVecRef,
                             IntoCompactListTree, FromCompactListTree,
                             IntoCompositeListTree, FromCompositeListTree};
pub use variable::MaxVec;
#[cfg(feature = "derive")]
pub use bm_le_derive::{FromTree, IntoTree};

/// End value for 256-bit ssz binary merkle tree.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct End(pub [u8; 32]);

impl Default for End {
    fn default() -> Self {
        Self([0; 32])
    }
}

impl AsRef<[u8]> for End {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<GenericArray<u8, typenum::U32>> for End {
    fn from(array: GenericArray<u8, typenum::U32>) -> Self {
        let mut ret = [0u8; 32];
        ret.copy_from_slice(array.as_slice());
        Self(ret)
    }
}

impl Into<GenericArray<u8, typenum::U32>> for End {
    fn into(self) -> GenericArray<u8, typenum::U32> {
        GenericArray::from_exact_iter(self.0.into_iter().cloned()).expect("Size equals to U32; qed")
    }
}

/// Intermediate type for 256-bit ssz binary merkle tree.
pub type Intermediate = GenericArray<u8, U32>;

/// Special type for le-compatible construct.
pub trait CompatibleConstruct: Construct<Intermediate=Intermediate, End=End> { }

impl<C: Construct<Intermediate=Intermediate, End=End>> CompatibleConstruct for C { }

/// Traits for type converting into a tree structure.
pub trait IntoTree {
    /// Convert this type into merkle tree, writing nodes into the
    /// given database.
    fn into_tree<DB: WriteBackend>(
        &self,
        db: &mut DB
    ) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct;
}

/// Traits for type converting from a tree structure.
pub trait FromTree: Sized {
    /// Convert this type from merkle tree, reading nodes from the
    /// given database.
    fn from_tree<DB: ReadBackend>(
        root: &ValueOf<DB::Construct>,
        db: &mut DB
    ) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct;
}

/// Indicate that the current value should be serialized and
/// deserialized in Compact format. Reference form.
#[derive(Debug, Eq, PartialEq)]
pub struct CompactRef<'a, T>(pub &'a T);

/// Indicate that the current value should be serialized and
/// deserialized in Compact format. Value form.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Compact<T>(pub T);

impl<T> From<T> for Compact<T> {
    fn from(t: T) -> Self {
        Self(t)
    }
}

/// Calculate a ssz merkle tree root, dismissing the tree.
pub fn tree_root<D, T>(value: &T) -> H256 where
    T: IntoTree,
    D: Digest<OutputSize=U32>,
{
    value.into_tree(&mut NoopBackend::<InheritedDigestConstruct<D, End>>::default())
        .map(|ret| H256::from_slice(ret.as_ref()))
        .expect("Noop backend never fails in set; qed")
}
