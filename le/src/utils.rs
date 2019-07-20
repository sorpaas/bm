//! Utilities

use bm::{ValueOf, ReadBackend, WriteBackend, Error};
use primitive_types::U256;
use crate::{CompatibleConstruct, IntoTree, FromTree};

pub use bm::utils::*;

/// Mix in type.
pub fn mix_in_type<T, DB: WriteBackend>(value: &T, db: &mut DB, ty: usize) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
    T: IntoTree,
    DB::Construct: CompatibleConstruct,
{
    let left = value.into_tree(db)?;
    let right = U256::from(ty).into_tree(db)?;

    (left, right).into_tree(db)
}

/// Decode type.
pub fn decode_with_type<DB: ReadBackend, F, R>(root: &ValueOf<DB::Construct>, db: &mut DB, f: F) -> Result<R, Error<DB::Error>> where
    F: FnOnce(&ValueOf<DB::Construct>, &mut DB, usize) -> Result<R, Error<DB::Error>>,
    DB::Construct: CompatibleConstruct,
{
    let (value, ty) = <(ValueOf<DB::Construct>, U256)>::from_tree(root, db)?;

    if ty > U256::from(usize::max_value()) {
        Err(Error::CorruptedDatabase)
    } else {
        f(&value, db, ty.as_usize())
    }
}

/// Mix in length.
pub fn mix_in_length<T, DB: WriteBackend>(value: &T, db: &mut DB, len: usize) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
    T: IntoTree,
    DB::Construct: CompatibleConstruct,
{
    let left = value.into_tree(db)?;
    let right = U256::from(len).into_tree(db)?;

    (left, right).into_tree(db)
}

/// Decode length.
pub fn decode_with_length<T, DB: ReadBackend>(root: &ValueOf<DB::Construct>, db: &mut DB) -> Result<(T, usize), Error<DB::Error>> where
    T: FromTree,
    DB::Construct: CompatibleConstruct,
{
    let (value, len) = <(T, U256)>::from_tree(root, db)?;

    if len > U256::from(usize::max_value()) {
        Err(Error::CorruptedDatabase)
    } else {
        Ok((value, len.as_usize()))
    }
}
