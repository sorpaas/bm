use bm::{Error, ValueOf, Value, Backend, Index, DanglingRaw, Leak};
use primitive_types::U256;
use alloc::vec::Vec;

use crate::{Composite, FixedVec, FromVectorTree, FromVectorTreeWithConfig, FixedVecRef, FromTree, End, Intermediate, IntoVectorTree, IntoTree};

/// Implement FromListTreeWithConfig for traits that has already
/// implemented FromListTree and does not need extra configs.
#[macro_export]
macro_rules! impl_from_list_tree_with_empty_config {
    ( $t:ty ) => {
        impl<C, DB> $crate::FromListTreeWithConfig<C, DB> for $t where
            DB: $crate::Backend<Intermediate=Intermediate, End=End>
        {
            fn from_list_tree_with_config(
                root: &$crate::ValueOf<DB>,
                db: &DB,
                max_len: Option<usize>,
                _config: &C,
            ) -> Result<Self, $crate::Error<DB::Error>> {
                <$t>::from_list_tree(root, db, max_len)
            }
        }
    }
}

/// Traits for list converting from a tree structure.
pub trait FromListTree<DB: Backend<Intermediate=Intermediate, End=End>>: Sized {
    /// Convert this type from merkle tree, reading nodes from the
    /// given database, with given maximum length.
    fn from_list_tree(
        root: &ValueOf<DB>,
        db: &DB,
        max_len: Option<usize>,
    ) -> Result<Self, Error<DB::Error>>;
}

/// Traits for list converting from a tree structure with config.
pub trait FromListTreeWithConfig<C, DB: Backend<Intermediate=Intermediate, End=End>>: Sized {
    /// Convert this type from merkle tree, reading nodes from the
    /// given database, with given maximum length.
    fn from_list_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        max_len: Option<usize>,
        config: &C,
    ) -> Result<Self, Error<DB::Error>>;
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` reference. In `ssz`'s definition, this is a "list".
pub struct VariableVecRef<'a, T>(pub &'a [T], pub Option<usize>);
#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` value. In `ssz`'s definition, this is a "list".
pub struct VariableVec<T>(pub Vec<T>, pub Option<usize>);

macro_rules! impl_packed {
    ( $t:ty, $len:expr ) => {
        impl<'a, DB> IntoTree<DB> for VariableVecRef<'a, $t> where
            DB: Backend<Intermediate=Intermediate, End=End>,
        {
            fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
                let max_len = self.1;
                let len = self.0.len();

                let left = FixedVecRef(&self.0).into_vector_tree(db, max_len)?;
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

        let left = FixedVecRef(&self.0).into_vector_tree(db, max_len)?;
        let right = U256::from(len).into_tree(db)?;
        let key = db.intermediate_of(&left, &right);

        db.insert(key.clone(), (left, right))?;
        Ok(Value::Intermediate(key))
    }
}

fn from_list_tree<T, F, DB>(
    root: &ValueOf<DB>,
    db: &DB,
    max_len: Option<usize>,
    f: F
) -> Result<VariableVec<T>, Error<DB::Error>> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    F: FnOnce(&ValueOf<DB>, &DB, usize, Option<usize>) -> Result<FixedVec<T>, Error<DB::Error>>
{
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

    let vector = f(
        &vector_root, db, len, max_len
    )?;

    Ok(VariableVec(vector.0, max_len))
}

impl<DB, T> FromListTree<DB> for VariableVec<T> where
    FixedVec<T>: FromVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_list_tree(
        root: &ValueOf<DB>,
        db: &DB,
        max_len: Option<usize>,
    ) -> Result<Self, Error<DB::Error>> {
        from_list_tree(root, db, max_len, |vector_root, db, len, max_len| {
            FixedVec::<T>::from_vector_tree(
                &vector_root, db, len, max_len
            )
        })
    }
}

impl<C, DB, T> FromListTreeWithConfig<C, DB> for VariableVec<T> where
    FixedVec<T>: FromVectorTreeWithConfig<C, DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_list_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        max_len: Option<usize>,
        config: &C,
    ) -> Result<Self, Error<DB::Error>> {
        from_list_tree(root, db, max_len, |vector_root, db, len, max_len| {
            FixedVec::<T>::from_vector_tree_with_config(
                &vector_root, db, len, max_len, config
            )
        })
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

impl<DB, T> IntoTree<DB> for [T] where
    for<'a> VariableVecRef<'a, T>: IntoTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        VariableVecRef(&self, None).into_tree(db)
    }
}

impl<DB, T> IntoTree<DB> for Vec<T> where
    for<'a> VariableVecRef<'a, T>: IntoTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        VariableVecRef(&self, None).into_tree(db)
    }
}

impl<DB, T> FromTree<DB> for Vec<T> where
    VariableVec<T>: FromListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        VariableVec::from_list_tree(root, db, None).map(|ret| ret.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bm::InMemoryBackend;
    use sha2::Sha256;

    #[test]
    fn test_plain() {
        let data = {
            let mut ret = Vec::new();
            for i in 0..17u16 {
                ret.push(i);
            }
            ret
        };

        let mut db = InMemoryBackend::<Sha256, End>::new_with_inherited_empty();
        let encoded = data.into_tree(&mut db).unwrap();
        let decoded = Vec::<u16>::from_tree(&encoded, &db).unwrap();
        assert_eq!(data, decoded);
    }
}
