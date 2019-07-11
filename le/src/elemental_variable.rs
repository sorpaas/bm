use bm::{Error, ValueOf, Value, Backend, Index, DanglingRaw, Leak};
use primitive_types::U256;
use alloc::vec::Vec;

use crate::{Composite, ElementalFixedVec, FromVectorTree, ElementalFixedVecRef, End, Intermediate, IntoVectorTree, IntoTree};

/// Traits for list converting into a tree structure.
pub trait IntoListTree<DB: Backend<Intermediate=Intermediate, End=End>> {
    /// Convert this list into merkle tree, writing nodes into the
    /// given database, and using the maximum length specified.
    fn into_list_tree(
        &self,
        db: &mut DB,
        max_len: Option<usize>
    ) -> Result<ValueOf<DB>, Error<DB::Error>>;
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

#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` reference. In `ssz`'s definition, this is a "list".
pub struct ElementalVariableVecRef<'a, T>(pub &'a [T]);
#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` value. In `ssz`'s definition, this is a "list".
pub struct ElementalVariableVec<T>(pub Vec<T>);

macro_rules! impl_packed {
    ( $t:ty, $len:expr ) => {
        impl<'a, DB> IntoListTree<DB> for ElementalVariableVecRef<'a, $t> where
            DB: Backend<Intermediate=Intermediate, End=End>,
        {
            fn into_list_tree(
                &self,
                db: &mut DB,
                max_len: Option<usize>
            ) -> Result<ValueOf<DB>, Error<DB::Error>> {
                let len = self.0.len();

                let left = ElementalFixedVecRef(&self.0).into_vector_tree(db, max_len)?;
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

impl<'a, DB, T: Composite> IntoListTree<DB> for ElementalVariableVecRef<'a, T> where
    for<'b> ElementalFixedVecRef<'b, T>: IntoVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_list_tree(
        &self,
        db: &mut DB,
        max_len: Option<usize>
    ) -> Result<ValueOf<DB>, Error<DB::Error>> {
        let len = self.0.len();

        let left = ElementalFixedVecRef(&self.0).into_vector_tree(db, max_len)?;
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
) -> Result<ElementalVariableVec<T>, Error<DB::Error>> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    F: FnOnce(&ValueOf<DB>, &DB, usize, Option<usize>) -> Result<ElementalFixedVec<T>, Error<DB::Error>>
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

    Ok(ElementalVariableVec(vector.0))
}

impl<DB, T> FromListTree<DB> for ElementalVariableVec<T> where
    ElementalFixedVec<T>: FromVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_list_tree(
        root: &ValueOf<DB>,
        db: &DB,
        max_len: Option<usize>,
    ) -> Result<Self, Error<DB::Error>> {
        from_list_tree(root, db, max_len, |vector_root, db, len, max_len| {
            ElementalFixedVec::<T>::from_vector_tree(
                &vector_root, db, len, max_len
            )
        })
    }
}

impl<DB, T> IntoListTree<DB> for ElementalVariableVec<T> where
    for<'a> ElementalVariableVecRef<'a, T>: IntoListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_list_tree(
        &self,
        db: &mut DB,
        max_len: Option<usize>
    ) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(&self.0).into_list_tree(db, max_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FromTree;

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
