use typenum::U32;
use generic_array::GenericArray;
use primitive_types::H256;
use digest::Digest;
use bm::{Backend, NoopBackend, Error, ValueOf};

mod basic;
mod fixed;
mod variable;

pub use fixed::{FixedVec, FixedVecRef, IntoVectorTree, FromVectorTree};
pub use variable::{VariableVec, VariableVecRef, FromListTree};

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

pub type Intermediate = GenericArray<u8, U32>;

/// Serializable type of merkle.
pub trait IntoTree<DB: Backend<Intermediate=Intermediate, End=End>> {
    /// Serialize this value into a list of merkle value.
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>>;
}

pub trait FromTree<DB: Backend<Intermediate=Intermediate, End=End>>: Sized {
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>>;
}

pub trait Composite { }

impl<'a, T> Composite for FixedVecRef<'a, T> { }
impl<T> Composite for FixedVec<T> { }
impl<'a, T> Composite for VariableVecRef<'a, T> { }
impl<T> Composite for VariableVec<T> { }
impl Composite for H256 { }

pub fn tree_root<D, T>(value: &T) -> H256 where
    T: IntoTree<NoopBackend<D, End>>,
    D: Digest<OutputSize=U32>,
{
    value.into_tree(&mut NoopBackend::new_with_inherited_empty())
        .map(|ret| H256::from_slice(ret.as_ref()))
        .expect("Noop backend never fails in set; qed")
}
