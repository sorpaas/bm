use bm::{Error, ValueOf, Value, Backend};
use bm::serialize::{self, Serialize};
use primitive_types::U256;

use crate::{Serial, FixedVecRef, End, Intermediate, SerializeVector};

pub struct VariableVecRef<'a, T>(pub &'a Vec<T>, pub usize);
pub struct VariableVec<T>(pub Vec<T>, pub usize);

impl<'a, 'b, DB, T> Serialize<DB> for Serial<'a, VariableVecRef<'b, T>> where
    for<'c, 'd> Serial<'c, FixedVecRef<'d, T>>: SerializeVector<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn serialize(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        let target_depth = serialize::required_depth((self.0).1);
        let len = (self.0).0.len();

        let left = Serial(&FixedVecRef(&(self.0).0)).serialize_vector(db, Some(target_depth))?;
        let right = Serial(&U256::from(len)).serialize(db)?;
        let key = db.intermediate_of(&left, &right);

        db.insert(key.clone(), (left, right))?;
        Ok(Value::Intermediate(key))
    }
}
