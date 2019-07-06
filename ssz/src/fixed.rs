use bm::{ValueOf, Backend, Error, Value};
use bm::serialize::{self, Serialize};
use primitive_types::{U256, H256};

use crate::{Serial, Intermediate, End, Composite};

pub trait SerializeVector<DB: Backend> {
    fn serialize_vector(&self, db: &mut DB, at_depth: Option<usize>) -> Result<ValueOf<DB>, Error<DB::Error>>;
}

pub struct FixedVecRef<'a, T>(pub &'a Vec<T>);
pub struct FixedVec<T>(pub Vec<T>);

macro_rules! impl_builtin_fixed_uint_vector {
    ( $( $t:ty ),* ) => { $(
        impl<'a, 'b, DB> SerializeVector<DB> for Serial<'a, FixedVecRef<'b, $t>> where
            DB: Backend<Intermediate=Intermediate, End=End>
        {
            fn serialize_vector(&self, db: &mut DB, at_depth: Option<usize>) -> Result<ValueOf<DB>, Error<DB::Error>> {
                let mut chunks: Vec<Vec<u8>> = Vec::new();

                for value in (self.0).0 {
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
                }).collect::<Vec<_>>(), db, at_depth)
            }
        }
    )* }
}

impl_builtin_fixed_uint_vector!(u8, u16, u32, u64, u128);

impl<'a, 'b, DB> SerializeVector<DB> for Serial<'a, FixedVecRef<'b, U256>> where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn serialize_vector(&self, db: &mut DB, at_depth: Option<usize>) -> Result<ValueOf<DB>, Error<DB::Error>> {
        serialize::serialize_vector(&(self.0).0.iter().map(|uint| {
            let mut ret = End::default();
            uint.to_little_endian(&mut ret.0);
            Value::End(ret)
        }).collect::<Vec<_>>(), db, at_depth)
    }
}

impl<'a, 'b, DB> SerializeVector<DB> for Serial<'a, FixedVecRef<'b, bool>> where
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn serialize_vector(&self, db: &mut DB, _at_depth: Option<usize>) -> Result<ValueOf<DB>, Error<DB::Error>> {
        let mut bytes = Vec::new();
        bytes.resize(((self.0).0.len() + 7) / 8, 0u8);

        for i in 0..(self.0).0.len() {
            bytes[i / 8] |= ((self.0).0[i] as u8) << (i % 8);
        }

        Serial(&FixedVec(bytes)).serialize(db)
    }
}

impl<'a, 'b, T: Composite, DB> SerializeVector<DB> for Serial<'a, FixedVecRef<'b, T>> where
    for<'c> Serial<'c, T>: Serialize<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn serialize_vector(&self, db: &mut DB, at_depth: Option<usize>) -> Result<ValueOf<DB>, Error<DB::Error>> {
        serialize::serialize_vector(&(self.0).0.iter().map(|value| {
            Serial(value).serialize(db)
        }).collect::<Result<Vec<_>, _>>()?, db, at_depth)
    }
}

impl<'a, DB> SerializeVector<DB> for Serial<'a, H256> where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn serialize_vector(&self, db: &mut DB, at_depth: Option<usize>) -> Result<ValueOf<DB>, Error<DB::Error>> {
        Serial(&FixedVecRef(&self.0.as_ref().iter().cloned().collect())).serialize_vector(db, at_depth)
    }
}

impl<'a, DB, T> SerializeVector<DB> for Serial<'a, FixedVec<T>> where
    for<'b, 'c> Serial<'b, FixedVecRef<'c, T>>: SerializeVector<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn serialize_vector(&self, db: &mut DB, at_depth: Option<usize>) -> Result<ValueOf<DB>, Error<DB::Error>> {
        Serial(&FixedVecRef(&(self.0).0)).serialize_vector(db, at_depth)
    }
}

impl<'a, DB, T> Serialize<DB> for Serial<'a, FixedVec<T>> where
    Self: SerializeVector<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn serialize(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        self.serialize_vector(db, None)
    }
}
