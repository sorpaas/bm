//! Utilities

use bm::{ValueOf, Backend, Error};
use primitive_types::U256;
use crate::{IntoTree, FromTree, Intermediate, End};

pub use bm::utils::*;

/// Mix in type.
pub fn mix_in_type<T, DB>(value: &T, db: &mut DB, ty: usize) -> Result<ValueOf<DB>, Error<DB::Error>> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    T: IntoTree<DB>,
{
    let left = value.into_tree(db)?;
    let right = U256::from(ty).into_tree(db)?;

    (left, right).into_tree(db)
}

/// Decode type.
pub fn decode_with_type<DB, F, R>(root: &ValueOf<DB>, db: &mut DB, f: F) -> Result<R, Error<DB::Error>> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    F: FnOnce(&ValueOf<DB>, &mut DB, usize) -> Result<R, Error<DB::Error>>,
{
    let (value, ty) = <(ValueOf<DB>, U256)>::from_tree(root, db)?;

    if ty > U256::from(usize::max_value()) {
        Err(Error::CorruptedDatabase)
    } else {
        f(&value, db, ty.as_usize())
    }
}

/// Mix in length.
pub fn mix_in_length<T, DB>(value: &T, db: &mut DB, len: usize) -> Result<ValueOf<DB>, Error<DB::Error>> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    T: IntoTree<DB>,
{
    let left = value.into_tree(db)?;
    let right = U256::from(len).into_tree(db)?;

    (left, right).into_tree(db)
}

/// Decode length.
pub fn decode_with_length<T, DB>(root: &ValueOf<DB>, db: &mut DB) -> Result<(T, usize), Error<DB::Error>> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    T: FromTree<DB>,
{
    let (value, len) = <(T, U256)>::from_tree(root, db)?;

    if len > U256::from(usize::max_value()) {
        Err(Error::CorruptedDatabase)
    } else {
        Ok((value, len.as_usize()))
    }
}
