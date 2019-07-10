use typenum::Unsigned;
use bm::{Error, ValueOf, Backend};
use core::marker::PhantomData;
use crate::{ElementalVariableVecRef, ElementalVariableVec, Intermediate, End, IntoTree, IntoListTree, FromTree, FromListTree, FromTreeWithConfig, Composite};

/// Traits for getting the maximum length from config.
pub trait MaxLenFromConfig<C> {
    /// Get the maximum length from config parameter.
    fn max_len_from_config(config: &C) -> Option<usize>;
}

/// Indicate a type that does not have maximum length.
pub struct NoMaxLen;

/// Trait indicate `MaxLenFromConfig` has a known maximum length.
pub trait KnownMaxLen {
    /// Get the static maximum length.
    fn max_len() -> Option<usize>;
}

impl<U: Unsigned> KnownMaxLen for U {
    fn max_len() -> Option<usize> {
        Some(U::to_usize())
    }
}

impl KnownMaxLen for NoMaxLen {
    fn max_len() -> Option<usize> {
        None
    }
}

impl<C, U: KnownMaxLen> MaxLenFromConfig<C> for U {
    fn max_len_from_config(_config: &C) -> Option<usize> {
        U::max_len()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` reference.
pub struct VariableVecRef<'a, T, ML>(pub &'a [T], pub Option<usize>, pub PhantomData<ML>);
#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` value.
pub struct VariableVec<T, ML>(pub Vec<T>, pub Option<usize>, pub PhantomData<ML>);

impl<'a, T, ML> Composite for VariableVecRef<'a, T, ML> { }
impl<T, ML> Composite for VariableVec<T, ML> { }

impl<'a, DB, T, L> IntoTree<DB> for VariableVecRef<'a, T, L> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(self.0).into_list_tree(db, self.1)
    }
}

impl<DB, T, L> IntoTree<DB> for VariableVec<T, L> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(&self.0).into_list_tree(db, self.1)
    }
}

impl<DB, T, ML: KnownMaxLen> FromTree<DB> for VariableVec<T, ML> where
    for<'a> ElementalVariableVec<T>: FromListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalVariableVec::<T>::from_list_tree(root, db, ML::max_len())?;
        Ok(VariableVec(value.0, ML::max_len(), PhantomData))
    }
}

impl<DB, C, T, ML: MaxLenFromConfig<C>> FromTreeWithConfig<C, DB> for VariableVec<T, ML> where
    for<'a> ElementalVariableVec<T>: FromListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        config: &C
    ) -> Result<Self, Error<DB::Error>> {
        let max_len = ML::max_len_from_config(config);
        let value = ElementalVariableVec::<T>::from_list_tree(
            root,
            db,
            max_len,
        )?;
        Ok(VariableVec(value.0, max_len, PhantomData))
    }
}

impl<T> Composite for [T] { }

impl<DB, T> IntoTree<DB> for [T] where
    for<'a> ElementalVariableVecRef<'a, T>: IntoListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(&self).into_list_tree(db, None)
    }
}

impl<T> Composite for Vec<T> { }

impl<DB, T> IntoTree<DB> for Vec<T> where
    for<'a> ElementalVariableVecRef<'a, T>: IntoListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(&self).into_list_tree(db, None)
    }
}

impl<DB, T> FromTree<DB> for Vec<T> where
    ElementalVariableVec<T>: FromListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        ElementalVariableVec::from_list_tree(root, db, None).map(|ret| ret.0)
    }
}
