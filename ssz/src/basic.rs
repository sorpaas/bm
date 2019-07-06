use bm::{Value, Backend, ValueOf, Error};
use bm::serialize::Serialize;
use primitive_types::U256;

use crate::{Serial, End, Intermediate};

impl<'a, DB> Serialize<DB> for Serial<'a, bool> where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn serialize(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        match self.0 {
            true => Serial(&1u8).serialize(db),
            false => Serial(&0u8).serialize(db),
        }
    }
}

macro_rules! impl_builtin_uint {
    ( $( $t:ty ),* ) => { $(
        impl<'a, DB> Serialize<DB> for Serial<'a, $t> where
            DB: Backend<Intermediate=Intermediate, End=End>
        {
            fn serialize(&self, _db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
                let mut ret = [0u8; 32];
                let bytes = self.0.to_le_bytes();
                ret[..bytes.len()].copy_from_slice(&bytes);

                Ok(Value::End(End(ret)))
            }
        }
    )* }
}

impl_builtin_uint!(u8, u16, u32, u64, u128);

impl<'a, DB> Serialize<DB> for Serial<'a, U256> where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn serialize(&self, _db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        let mut ret = [0u8; 32];
        self.0.to_little_endian(&mut ret);

        Ok(Value::End(End(ret)))
    }
}
