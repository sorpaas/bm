use bm::{Error, ValueOf, Value, Backend, Index, DanglingRaw, Leak};
use primitive_types::U256;

use crate::{Composite, FixedVec, FromVectorTree, FixedVecRef, End, Intermediate, IntoVectorTree, IntoTree};

/// Traits for list converting from a tree structure.
pub trait FromListTree<DB: Backend<Intermediate=Intermediate, End=End>>: Sized {
    /// Convert this type from merkle tree, reading nodes from the
    /// given database, with given maximum length.
    fn from_list_tree(
        root: &ValueOf<DB>,
        db: &DB,
        max_len: usize,
    ) -> Result<Self, Error<DB::Error>>;
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` reference. In `ssz`'s definition, this is a "list".
pub struct VariableVecRef<'a, T>(pub &'a [T], pub usize);
#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` value. In `ssz`'s definition, this is a "list".
pub struct VariableVec<T>(pub Vec<T>, pub usize);

macro_rules! impl_packed {
    ( $t:ty, $len:expr ) => {
        impl<'a, DB> IntoTree<DB> for VariableVecRef<'a, $t> where
            DB: Backend<Intermediate=Intermediate, End=End>,
        {
            fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
                let max_len = self.1 * $len / 256;
                let len = self.0.len();

                let left = FixedVecRef(&self.0).into_vector_tree(db, Some(max_len))?;
                let right = U256::from(len).into_tree(db)?;
                let key = db.intermediate_of(&left, &right);

                db.insert(key.clone(), (left, right))?;
                Ok(Value::Intermediate(key))
            }
        }
    }
}

impl_packed!(bool, 1);
impl_packed!(u8, 8);
impl_packed!(u16, 16);
impl_packed!(u32, 32);
impl_packed!(u64, 64);
impl_packed!(u128, 128);
impl_packed!(U256, 256);

impl<'a, DB, T: Composite> IntoTree<DB> for VariableVecRef<'a, T> where
    for<'b> FixedVecRef<'b, T>: IntoVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        let max_len = self.1;
        let len = self.0.len();

        let left = FixedVecRef(&self.0).into_vector_tree(db, Some(max_len))?;
        let right = U256::from(len).into_tree(db)?;
        let key = db.intermediate_of(&left, &right);

        db.insert(key.clone(), (left, right))?;
        Ok(Value::Intermediate(key))
    }
}

impl<DB, T> FromListTree<DB> for VariableVec<T> where
    FixedVec<T>: FromVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_list_tree(
        root: &ValueOf<DB>,
        db: &DB,
        max_len: usize,
    ) -> Result<Self, Error<DB::Error>> {
        let raw = DanglingRaw::<DB>::from_leaked(root.clone());

        let vector_root = raw.get(db, Index::root().left())?.ok_or(Error::CorruptedDatabase)?;
        let len_raw = raw.get(db, Index::root().right())?.ok_or(Error::CorruptedDatabase)?
            .end().ok_or(Error::CorruptedDatabase)?;

        let len_big = U256::from_little_endian(&len_raw.0);
        let len = if len_big > U256::from(usize::max_value()) {
            return Err(Error::CorruptedDatabase)
        } else {
            len_big.as_usize()
        };

        let vector = FixedVec::<T>::from_vector_tree(
            &vector_root, db, len, Some(max_len)
        )?;

        Ok(Self(vector.0, max_len))
    }
}

impl<DB, T> IntoTree<DB> for VariableVec<T> where
    for<'a> VariableVecRef<'a, T>: IntoTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        VariableVecRef(&self.0, self.1).into_tree(db)
    }
}
