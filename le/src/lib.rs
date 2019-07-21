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
use core::marker::PhantomData;

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

/// Digest construct for bm-le.
pub struct DigestConstruct<D: Digest<OutputSize=U32>>(PhantomData<D>);

impl<D: Digest<OutputSize=U32>> Construct for DigestConstruct<D> {
    type Intermediate = Intermediate;
    type End = End;

    fn intermediate_of(left: &ValueOf<Self>, right: &ValueOf<Self>) -> Self::Intermediate {
        let mut digest = D::new();
        digest.input(&left.as_ref()[..]);
        digest.input(&right.as_ref()[..]);
        H256::from_slice(digest.result().as_slice())
    }

    fn empty_at<DB: WriteBackend<Construct=Self>>(
        db: &mut DB,
        depth_to_bottom: usize
    ) -> Result<ValueOf<Self>, DB::Error> {
        let mut current = Value::End(Default::default());
        for _ in 0..depth_to_bottom {
            let value = (current.clone(), current);
            let key = Self::intermediate_of(&value.0, &value.1);
            db.insert(key.clone(), value)?;
            current = Value::Intermediate(key);
        }
        Ok(current)
    }
}

/// End value for 256-bit ssz binary merkle tree.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "parity-codec", derive(parity_codec::Encode, parity_codec::Decode))]
pub struct End(pub H256);

impl Default for End {
    fn default() -> Self {
        Self(H256::default())
    }
}

impl AsRef<[u8]> for End {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<GenericArray<u8, typenum::U32>> for End {
    fn from(array: GenericArray<u8, typenum::U32>) -> Self {
        Self(H256::from_slice(array.as_slice()))
    }
}

impl Into<GenericArray<u8, typenum::U32>> for End {
    fn into(self) -> GenericArray<u8, typenum::U32> {
        GenericArray::from_exact_iter(self.0.as_ref().iter().cloned()).expect("Size equals to U32; qed")
    }
}

/// Intermediate type for 256-bit ssz binary merkle tree.
pub type Intermediate = H256;

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
    value.into_tree(&mut NoopBackend::<DigestConstruct<D>>::default())
        .map(|ret| H256::from_slice(ret.as_ref()))
        .expect("Noop backend never fails in set; qed")
}
