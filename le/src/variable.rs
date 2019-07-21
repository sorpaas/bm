use typenum::Unsigned;
use bm::{Error, ValueOf, ReadBackend, WriteBackend};
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use alloc::vec::Vec;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use crate::{ElementalVariableVecRef, ElementalVariableVec,
            IntoTree, IntoCompactListTree, IntoCompositeListTree,
            FromTree, FromCompactListTree, FromCompositeListTree,
            Compact, CompactRef, CompatibleConstruct};

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

impl<T, ML> AsRef<[T]> for MaxVec<T, ML> {
    fn as_ref(&self) -> &[T] {
        &self.0
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

impl<T, ML: Unsigned> IntoTree for MaxVec<T, ML> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoCompositeListTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalVariableVecRef(&self.0).into_composite_list_tree(db, Some(ML::to_usize()))
    }
}

impl<T, ML: Unsigned> FromTree for MaxVec<T, ML> where
    for<'a> ElementalVariableVec<T>: FromCompositeListTree,
{
    fn from_tree<DB: ReadBackend>(root: &ValueOf<DB::Construct>, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let value = ElementalVariableVec::<T>::from_composite_list_tree(
            root, db, Some(ML::to_usize())
        )?;
        Ok(MaxVec(value.0, PhantomData))
    }
}

impl<'a, T, ML: Unsigned> IntoTree for CompactRef<'a, MaxVec<T, ML>> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoCompactListTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalVariableVecRef(&self.0).into_compact_list_tree(db, Some(ML::to_usize()))
    }
}

impl<T, ML: Unsigned> IntoTree for Compact<MaxVec<T, ML>> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoCompactListTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalVariableVecRef(&self.0).into_compact_list_tree(db, Some(ML::to_usize()))
    }
}

impl<T, ML: Unsigned> FromTree for Compact<MaxVec<T, ML>> where
    for<'a> ElementalVariableVec<T>: FromCompactListTree,
{
    fn from_tree<DB: ReadBackend>(root: &ValueOf<DB::Construct>, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let value = ElementalVariableVec::<T>::from_compact_list_tree(
            root, db, Some(ML::to_usize())
        )?;
        Ok(Self(MaxVec(value.0, PhantomData)))
    }
}

impl<T> IntoTree for [T] where
    for<'a> ElementalVariableVecRef<'a, T>: IntoCompositeListTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalVariableVecRef(&self).into_composite_list_tree(db, None)
    }
}

impl<T> IntoTree for Vec<T> where
    for<'a> ElementalVariableVecRef<'a, T>: IntoCompositeListTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalVariableVecRef(&self).into_composite_list_tree(db, None)
    }
}

impl<T> FromTree for Vec<T> where
    ElementalVariableVec<T>: FromCompositeListTree,
{
    fn from_tree<DB: ReadBackend>(root: &ValueOf<DB::Construct>, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalVariableVec::from_composite_list_tree(root, db, None).map(|ret| ret.0)
    }
}
