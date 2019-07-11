use core::ops::{Deref, DerefMut};
use bm::{Backend, ValueOf, Error};
use primitive_types::H256;
use generic_array::{GenericArray, ArrayLength};
use crate::{ElementalFixedVecRef, ElementalFixedVec, IntoCompositeVectorTree,
            IntoCompactVectorTree, IntoTree, FromTree, FromCompositeVectorTree,
            FromCompactVectorTree, Intermediate, End};

#[derive(Debug, Clone, Eq, PartialEq, Default)]
/// Compact array.
pub struct CompactArray<T, L: ArrayLength<T>>(pub GenericArray<T, L>);

impl<T, L: ArrayLength<T>> Deref for CompactArray<T, L> {
    type Target = GenericArray<T, L>;

    fn deref(&self) -> &GenericArray<T, L> {
        &self.0
    }
}

impl<T, L: ArrayLength<T>> DerefMut for CompactArray<T, L> {
    fn deref_mut(&mut self) -> &mut GenericArray<T, L> {
        &mut self.0
    }
}

impl<T, L: ArrayLength<T>> From<GenericArray<T, L>> for CompactArray<T, L> {
    fn from(array: GenericArray<T, L>) -> Self {
        Self(array)
    }
}

impl<T, L: ArrayLength<T>, DB> IntoTree<DB> for CompactArray<T, L> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    for<'a> ElementalFixedVecRef<'a, T>: IntoCompactVectorTree<DB>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self.0).into_compact_vector_tree(db, None)
    }
}

impl<T, L: ArrayLength<T>, DB> FromTree<DB> for CompactArray<T, L> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    T: Default,
    ElementalFixedVec<T>: FromCompactVectorTree<DB>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<T>::from_compact_vector_tree(root, db, L::to_usize(), None)?;
        let mut ret = GenericArray::default();
        for (i, v) in value.0.into_iter().enumerate() {
            ret[i] = v;
        }
        Ok(Self(ret))
    }
}

impl<DB> IntoTree<DB> for H256 where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self.0.as_ref()).into_compact_vector_tree(db, None)
    }
}

impl<DB> FromTree<DB> for H256 where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<u8>::from_compact_vector_tree(root, db, 32, None)?;
        Ok(Self::from_slice(value.0.as_ref()))
    }
}

macro_rules! impl_fixed_array {
    ( $( $n:expr ),* ) => { $(
        impl<DB, T> IntoTree<DB> for [T; $n] where
            DB: Backend<Intermediate=Intermediate, End=End>,
            for<'a> ElementalFixedVecRef<'a, T>: IntoCompositeVectorTree<DB>,
        {
            fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
                ElementalFixedVecRef(&self[..]).into_composite_vector_tree(db, None)
            }
        }

        impl<DB, T> FromTree<DB> for [T; $n] where
            DB: Backend<Intermediate=Intermediate, End=End>,
            T: Default + Copy,
            for<'a> ElementalFixedVec<T>: FromCompositeVectorTree<DB>,
        {
            fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
                let value = ElementalFixedVec::<T>::from_composite_vector_tree(root, db, $n, None)?;
                let mut ret = [T::default(); $n];
                for (i, v) in value.0.into_iter().enumerate() {
                    ret[i] = v;
                }
                Ok(ret)
            }
        }
    )* }
}

impl_fixed_array!(1, 2, 3, 4, 5, 6, 7, 8,
                  9, 10, 11, 12, 13, 14, 15, 16,
                  17, 18, 19, 20, 21, 22, 23, 24,
                  25, 26, 27, 28, 29, 30, 31, 32);

impl<DB, T, L: ArrayLength<T>> IntoTree<DB> for GenericArray<T, L> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    for<'a> ElementalFixedVecRef<'a, T>: IntoCompositeVectorTree<DB>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self[..]).into_composite_vector_tree(db, None)
    }
}

impl<DB, T, L: ArrayLength<T>> FromTree<DB> for GenericArray<T, L> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    for<'a> ElementalFixedVec<T>: FromCompositeVectorTree<DB>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<T>::from_composite_vector_tree(root, db, L::to_usize(), None)?;
        Ok(GenericArray::from_exact_iter(value.0)
           .expect("Fixed vec must build vector with L::as_usize; qed"))
    }
}
