use core::marker::PhantomData;
use bm::{Backend, ValueOf, Error};
use primitive_types::H256;
use generic_array::{GenericArray, ArrayLength};
use typenum::Unsigned;
use crate::{impl_from_tree_with_empty_config, ElementalFixedVecRef, ElementalFixedVec, IntoVectorTree, IntoTree, FromTree, FromTreeWithConfig, FromVectorTree, Intermediate, End, Composite};

/// Traits for getting the length from config.
pub trait LenFromConfig<C> {
    /// Get the length from config parameter.
    fn len_from_config(config: &C) -> usize;
}

impl<C, U: Unsigned> LenFromConfig<C> for U {
    fn len_from_config(_config: &C) -> usize {
        U::to_usize()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Fixed `Vec` reference. In ssz's definition, this is a "vector".
pub struct FixedVecRef<'a, T, L>(pub &'a [T], pub PhantomData<L>);
#[derive(Debug, Clone, Eq, PartialEq)]
/// Fixed `Vec` value. In ssz's definition, this is a "vector".
pub struct FixedVec<T, L>(pub Vec<T>, pub PhantomData<L>);

impl<'a, T, L> Composite for FixedVecRef<'a, T, L> { }
impl<T, L> Composite for FixedVec<T, L> { }

impl<'a, DB, T, L> IntoTree<DB> for FixedVecRef<'a, T, L> where
    for<'b> ElementalFixedVecRef<'b, T>: IntoVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(self.0).into_vector_tree(db, None)
    }
}

impl<DB, T, L> IntoTree<DB> for FixedVec<T, L> where
    for<'b> ElementalFixedVecRef<'b, T>: IntoVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self.0).into_vector_tree(db, None)
    }
}

impl<DB, T, L: Unsigned> FromTree<DB> for FixedVec<T, L> where
    for<'a> ElementalFixedVec<T>: FromVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<T>::from_vector_tree(root, db, L::to_usize(), None)?;
        Ok(FixedVec(value.0, PhantomData))
    }
}

impl<DB, C, T, L: LenFromConfig<C>> FromTreeWithConfig<C, DB> for FixedVec<T, L> where
    for<'a> ElementalFixedVec<T>: FromVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        config: &C
    ) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<T>::from_vector_tree(
            root,
            db,
            L::len_from_config(config),
            None
        )?;
        Ok(FixedVec(value.0, PhantomData))
    }
}

impl Composite for H256 { }

impl<DB> IntoTree<DB> for H256 where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self.0.as_ref()).into_vector_tree(db, None)
    }
}

impl_from_tree_with_empty_config!(H256);
impl<DB> FromTree<DB> for H256 where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<u8>::from_vector_tree(root, db, 32, None)?;
        Ok(Self::from_slice(value.0.as_ref()))
    }
}

macro_rules! impl_fixed_array {
    ( $( $n:expr ),* ) => { $(
        impl<T> Composite for [T; $n] { }

        impl<DB, T> IntoTree<DB> for [T; $n] where
            DB: Backend<Intermediate=Intermediate, End=End>,
            for<'a> ElementalFixedVecRef<'a, T>: IntoTree<DB>,
        {
            fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
                ElementalFixedVecRef(&self[..]).into_tree(db)
            }
        }

        // This is similar to `impl_from_tree_with_empty_config!([T; $n])`
        // but we cannot use it directly.
        impl<DB, T, C> FromTreeWithConfig<C, DB> for [T; $n] where
            DB: Backend<Intermediate=Intermediate, End=End>,
            T: Default + Copy,
            for<'a> ElementalFixedVec<T>: FromVectorTree<DB>,
        {
            fn from_tree_with_config(
                root: &ValueOf<DB>,
                db: &DB,
                _config: &C
            ) -> Result<Self, Error<DB::Error>> {
                FromTree::from_tree(root, db)
            }
        }

        impl<DB, T> FromTree<DB> for [T; $n] where
            DB: Backend<Intermediate=Intermediate, End=End>,
            T: Default + Copy,
            for<'a> ElementalFixedVec<T>: FromVectorTree<DB>,
        {
            fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
                let value = ElementalFixedVec::<T>::from_vector_tree(root, db, $n, None)?;
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

impl<T, L: ArrayLength<T>> Composite for GenericArray<T, L> { }

impl<DB, T, L: ArrayLength<T>> IntoTree<DB> for GenericArray<T, L> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    for<'a> ElementalFixedVecRef<'a, T>: IntoTree<DB>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self[..]).into_tree(db)
    }
}

// This is similar to `impl_from_tree_with_empty_config!(GenericArray<T, L>)`
// but we cannot use it directly.
impl<DB, T, L: ArrayLength<T>, C> FromTreeWithConfig<C, DB> for GenericArray<T, L> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    for<'a> ElementalFixedVec<T>: FromVectorTree<DB>,
{
    fn from_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        _config: &C
    ) -> Result<Self, Error<DB::Error>> {
        FromTree::from_tree(root, db)
    }
}

impl<DB, T, L: ArrayLength<T>> FromTree<DB> for GenericArray<T, L> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    for<'a> ElementalFixedVec<T>: FromVectorTree<DB>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<T>::from_vector_tree(root, db, L::to_usize(), None)?;
        Ok(GenericArray::from_exact_iter(value.0)
           .expect("Fixed vec must build vector with L::as_usize; qed"))
    }
}
