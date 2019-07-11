use typenum::Unsigned;
use bm::{Error, ValueOf, Backend};
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use crate::{ElementalVariableVecRef, ElementalVariableVec, Intermediate, End, IntoTree, IntoListTree, FromTree, FromListTree, Composite};

#[derive(Debug, Clone, Eq, PartialEq)]
/// Vec reference with maximum length.
pub struct MaxVecRef<'a, T, ML>(pub &'a [T], PhantomData<ML>);
#[derive(Debug, Clone, Eq, PartialEq)]
/// Vec value with maximum length.
pub struct MaxVec<T, ML>(pub Vec<T>, PhantomData<ML>);

impl<'a, T, ML> Deref for MaxVecRef<'a, T, ML> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.0
    }
}

impl<T, ML> Deref for MaxVec<T, ML> {
    type Target = Vec<T>;

    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T, ML> DerefMut for MaxVec<T, ML> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T, ML> Default for MaxVec<T, ML> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<T, ML> From<Vec<T>> for MaxVec<T, ML> {
    fn from(vec: Vec<T>) -> Self {
        Self(vec, PhantomData)
    }
}

impl<'a, T, ML> Composite for MaxVecRef<'a, T, ML> { }
impl<T, ML> Composite for MaxVec<T, ML> { }

impl<'a, DB, T, ML: Unsigned> IntoTree<DB> for MaxVecRef<'a, T, ML> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(self.0).into_list_tree(db, Some(ML::to_usize()))
    }
}

impl<DB, T, ML: Unsigned> IntoTree<DB> for MaxVec<T, ML> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(&self.0).into_list_tree(db, Some(ML::to_usize()))
    }
}

impl<DB, T, ML: Unsigned> FromTree<DB> for MaxVec<T, ML> where
    for<'a> ElementalVariableVec<T>: FromListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalVariableVec::<T>::from_list_tree(root, db, Some(ML::to_usize()))?;
        Ok(MaxVec(value.0, PhantomData))
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
