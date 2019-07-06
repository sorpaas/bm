use bm::{Error, ValueOf, Value, Backend};
use bm::serialize;
use primitive_types::U256;

use crate::{FixedVecRef, End, Intermediate, IntoVectorTree, IntoTree};

pub struct VariableVecRef<'a, T>(pub &'a [T], pub usize);
pub struct VariableVec<T>(pub Vec<T>, pub usize);

impl<'a, DB, T> IntoTree<DB> for VariableVecRef<'a, T> where
    for<'b> FixedVecRef<'b, T>: IntoVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        let target_depth = serialize::required_depth(self.1);
        let len = self.0.len();

        let left = FixedVecRef(&self.0).into_vector_tree(db, Some(target_depth))?;
        let right = U256::from(len).into_tree(db)?;
        let key = db.intermediate_of(&left, &right);

        db.insert(key.clone(), (left, right))?;
        Ok(Value::Intermediate(key))
    }
}
