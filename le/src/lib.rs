#![warn(missing_docs)]

//! SimpleSerialize (ssz) compliant binary merkle tree supporting both
//! merkleization and de-merkleization.

use typenum::U32;
use generic_array::GenericArray;
use primitive_types::H256;
use digest::Digest;
use bm::NoopBackend;

pub use bm::{Backend, Error, ValueOf, Value, Vector, List, utils};

mod basic;
mod fixed;
mod variable;

pub use fixed::{FixedVec, FixedVecRef, IntoVectorTree, FromVectorTree, FromVectorTreeWithConfig};
pub use variable::{VariableVec, VariableVecRef, FromListTree, FromListTreeWithConfig};

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

/// Traits for type converting into a tree structure.
pub trait IntoTree<DB: Backend<Intermediate=Intermediate, End=End>> {
    /// Convert this type into merkle tree, writing nodes into the
    /// given database.
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>>;
}

/// Traits for type converting from a tree structure.
pub trait FromTree<DB: Backend<Intermediate=Intermediate, End=End>>: Sized {
    /// Convert this type from merkle tree, reading nodes from the
    /// given database.
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>>;
}

/// Traits for type converting from a tree structure with a config.
pub trait FromTreeWithConfig<C, DB: Backend<Intermediate=Intermediate, End=End>>: Sized {
    /// Convert this type from merkle tree, reading nodes from the
    /// given database, with the given config.
    fn from_tree_with_config(root: &ValueOf<DB>, db: &DB, config: &C) -> Result<Self, Error<DB::Error>>;
}

impl<DB, C, T: FromTree<DB>> FromTreeWithConfig<C, DB> for T where
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree_with_config(root: &ValueOf<DB>, db: &DB, _config: &C) -> Result<Self, Error<DB::Error>> {
        T::from_tree(root, db)
    }
}

/// A composite value, in contrary to ssz's definition of basic value.
pub trait Composite { }

impl<'a, T> Composite for FixedVecRef<'a, T> { }
impl<T> Composite for FixedVec<T> { }
impl<'a, T> Composite for VariableVecRef<'a, T> { }
impl<T> Composite for VariableVec<T> { }
impl Composite for H256 { }

/// Calculate a ssz merkle tree root, dismissing the tree.
pub fn tree_root<D, T>(value: &T) -> H256 where
    T: IntoTree<NoopBackend<D, End>>,
    D: Digest<OutputSize=U32>,
{
    value.into_tree(&mut NoopBackend::new_with_inherited_empty())
        .map(|ret| H256::from_slice(ret.as_ref()))
        .expect("Noop backend never fails in set; qed")
}
