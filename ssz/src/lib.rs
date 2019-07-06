use typenum::U32;
use generic_array::GenericArray;
use primitive_types::H256;
use digest::Digest;
use bm::{Backend, NoopBackend, Error, ValueOf};

mod basic;
mod fixed;
mod variable;

pub use fixed::{FixedVec, FixedVecRef, IntoVectorTree};
pub use variable::{VariableVec, VariableVecRef};

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

pub type Intermediate = GenericArray<u8, U32>;

/// Serializable type of merkle.
pub trait IntoTree<DB: Backend<Intermediate=Intermediate, End=End>> {
    /// Serialize this value into a list of merkle value.
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>>;
}

pub trait Composite { }

impl<'a, T> Composite for FixedVecRef<'a, T> { }
impl<T> Composite for FixedVec<T> { }
impl<'a, T> Composite for VariableVecRef<'a, T> { }
impl<T> Composite for VariableVec<T> { }
impl Composite for H256 { }


pub fn into_tree<T, DB>(value: &T, db: &mut DB) -> Result<H256, Error<DB::Error>> where
    T: IntoTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    value.into_tree(db).map(|ret| H256::from_slice(ret.as_ref()))
}

pub fn tree_root<D, T>(value: &T) -> H256 where
    T: IntoTree<NoopBackend<D, End>>,
    D: Digest<OutputSize=U32>,
{
    into_tree(value, &mut NoopBackend::new_with_inherited_empty())
        .expect("Noop backend never fails in set; qed")
}
