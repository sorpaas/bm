use bm::{Value, Backend, ValueOf, Error, Index, DanglingRaw, Leak};
use primitive_types::U256;

use crate::{IntoTree, FromTree, End, Intermediate};
use crate::utils::{mix_in_type, decode_with_type};

impl<DB> IntoTree<DB> for bool where
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        match self {
            true => 1u8.into_tree(db),
            false => 0u8.into_tree(db),
        }
    }
}

impl<DB> FromTree<DB> for bool where
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        Ok(u8::from_tree(root, db)? != 0)
    }
}

macro_rules! impl_builtin_uint {
    ( $( $t:ty ),* ) => { $(
        impl<DB> IntoTree<DB> for $t where
            DB: Backend<Intermediate=Intermediate, End=End>,
        {
            fn into_tree(&self, _db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
                let mut ret = [0u8; 32];
                let bytes = self.to_le_bytes();
                ret[..bytes.len()].copy_from_slice(&bytes);

                Ok(Value::End(End(ret)))
            }
        }

        impl<DB> FromTree<DB> for $t where
            DB: Backend<Intermediate=Intermediate, End=End>,
        {
            fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
                let raw = DanglingRaw::from_leaked(root.clone());

                match raw.get(db, Index::root())?.ok_or(Error::CorruptedDatabase)? {
                    Value::Intermediate(_) => Err(Error::CorruptedDatabase),
                    Value::End(value) => {
                        let mut bytes = Self::default().to_le_bytes();
                        let bytes_len = bytes.len();
                        bytes.copy_from_slice(&value.0[..bytes_len]);

                        Ok(Self::from_le_bytes(bytes))
                    },
                }
            }
        }
    )* }
}

impl_builtin_uint!(u8, u16, u32, u64, u128);

impl<DB> IntoTree<DB> for U256 where
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, _db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        let mut ret = [0u8; 32];
        self.to_little_endian(&mut ret);

        Ok(Value::End(End(ret)))
    }
}

impl<DB> FromTree<DB> for U256 where
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let raw = DanglingRaw::from_leaked(root.clone());

        match raw.get(db, Index::root())?.ok_or(Error::CorruptedDatabase)? {
            Value::Intermediate(_) => Err(Error::CorruptedDatabase),
            Value::End(value) => {
                Ok(U256::from_little_endian(&value.0))
            },
        }
    }
}

impl<DB> IntoTree<DB> for ValueOf<DB> where
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        Ok(self.clone())
    }
}

impl<DB> FromTree<DB> for ValueOf<DB> where
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        Ok(root.clone())
    }
}

impl<T, DB> FromTree<DB> for Option<T> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    T: FromTree<DB>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        decode_with_type(root, db, |inner, db, ty| {
            match ty {
                0 => {
                    <()>::from_tree(inner, db)?;
                    Ok(None)
                },
                1 => Ok(Some(T::from_tree(inner, db)?)),
                _ => Err(Error::CorruptedDatabase),
            }
        })
    }
}

impl<T, DB> IntoTree<DB> for Option<T> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    T: IntoTree<DB>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        match self {
            None => mix_in_type(&(), db, 0),
            Some(value) => mix_in_type(value, db, 1),
        }
    }
}
