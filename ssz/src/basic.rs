use bm::{Value, Backend, ValueOf, Error};
use primitive_types::U256;

use crate::{IntoTree, End, Intermediate};

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
    )* }
}

impl_builtin_uint!(u8, u16, u32, u64, u128);

impl<'a, DB> IntoTree<DB> for U256 where
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, _db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        let mut ret = [0u8; 32];
        self.to_little_endian(&mut ret);

        Ok(Value::End(End(ret)))
    }
}
