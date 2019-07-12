use typenum::Unsigned;
use bm::{Error, ValueOf, Backend};
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use crate::{ElementalVariableVecRef, ElementalVariableVec, Intermediate, End,
            IntoTree, IntoCompactListTree, IntoCompositeListTree,
            FromTree, FromCompactListTree, FromCompositeListTree,
            Compact, CompactRef};

/// Vec value with maximum length.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "Vec<T>", into = "Vec<T>"))]
#[cfg_attr(feature = "serde", serde(bound = "T: Clone + Serialize + DeserializeOwned + 'static, ML: Clone"))]
pub struct MaxVec<T, ML>(pub Vec<T>, PhantomData<ML>);

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

impl<T, ML> Into<Vec<T>> for MaxVec<T, ML> {
    fn into(self) -> Vec<T> {
        self.0
    }
}

impl<DB, T, ML: Unsigned> IntoTree<DB> for MaxVec<T, ML> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoCompositeListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(&self.0).into_composite_list_tree(db, Some(ML::to_usize()))
    }
}

impl<DB, T, ML: Unsigned> FromTree<DB> for MaxVec<T, ML> where
    for<'a> ElementalVariableVec<T>: FromCompositeListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalVariableVec::<T>::from_composite_list_tree(
            root, db, Some(ML::to_usize())
        )?;
        Ok(MaxVec(value.0, PhantomData))
    }
}

impl<'a, DB, T, ML: Unsigned> IntoTree<DB> for CompactRef<'a, MaxVec<T, ML>> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoCompactListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(&self.0).into_compact_list_tree(db, Some(ML::to_usize()))
    }
}

impl<DB, T, ML: Unsigned> IntoTree<DB> for Compact<MaxVec<T, ML>> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoCompactListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(&self.0).into_compact_list_tree(db, Some(ML::to_usize()))
    }
}

impl<DB, T, ML: Unsigned> FromTree<DB> for Compact<MaxVec<T, ML>> where
    for<'a> ElementalVariableVec<T>: FromCompactListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalVariableVec::<T>::from_compact_list_tree(
            root, db, Some(ML::to_usize())
        )?;
        Ok(Self(MaxVec(value.0, PhantomData)))
    }
}

impl<DB, T> IntoTree<DB> for [T] where
    for<'a> ElementalVariableVecRef<'a, T>: IntoCompositeListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(&self).into_composite_list_tree(db, None)
    }
}

impl<DB, T> IntoTree<DB> for Vec<T> where
    for<'a> ElementalVariableVecRef<'a, T>: IntoCompositeListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(&self).into_composite_list_tree(db, None)
    }
}

impl<DB, T> FromTree<DB> for Vec<T> where
    ElementalVariableVec<T>: FromCompositeListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        ElementalVariableVec::from_composite_list_tree(root, db, None).map(|ret| ret.0)
    }
}
