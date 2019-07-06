use bm::{ValueOf, Backend, Error, Value};
use bm::serialize::{self, Serialize};
use primitive_types::{U256, H256};

use crate::{Serial, Intermediate, End, Composite};

pub struct FixedVec<T>(pub Vec<T>);

macro_rules! impl_builtin_fixed_uint_vector {
    ( $( $t:ty ),* ) => { $(
        impl<'a, DB> Serialize<DB> for Serial<'a, FixedVec<$t>> where
            DB: Backend<Intermediate=Intermediate, End=End>
        {
            fn serialize(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
                let mut chunks: Vec<Vec<u8>> = Vec::new();

                for value in &(self.0).0 {
                    if chunks.last().map(|v| v.len() == 32).unwrap_or(true) {
                        chunks.push(Vec::new());
                    }

                    let current = chunks.last_mut().expect("chunks must have at least one item; qed");
                    current.append(&mut value.to_le_bytes().into_iter().cloned().collect::<Vec<u8>>());
                }

                serialize::serialize_vector(&chunks.into_iter().map(|c| {
                    let mut ret = End::default();
                    ret.0.copy_from_slice(&c);
                    Value::End(ret)
                }).collect::<Vec<_>>(), db, None)
            }
        }
    )* }
}

impl_builtin_fixed_uint_vector!(u8, u16, u32, u64, u128);

impl<'a, DB> Serialize<DB> for Serial<'a, FixedVec<U256>> where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn serialize(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        serialize::serialize_vector(&(self.0).0.iter().map(|uint| {
            let mut ret = End::default();
            uint.to_little_endian(&mut ret.0);
            Value::End(ret)
        }).collect::<Vec<_>>(), db, None)
    }
}

impl<'a, DB> Serialize<DB> for Serial<'a, H256> where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn serialize(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        Serial(&FixedVec(self.0.as_ref().iter().cloned().collect())).serialize(db)
    }
}

impl<'a, T: Composite, DB> Serialize<DB> for Serial<'a, FixedVec<T>> where
    for<'b> Serial<'b, T>: Serialize<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn serialize(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        serialize::serialize_vector(&(self.0).0.iter().map(|value| {
            Serial(value).serialize(db)
        }).collect::<Result<Vec<_>, _>>()?, db, None)
    }
}
