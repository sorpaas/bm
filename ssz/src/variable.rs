use bm::{Error, ValueOf, Value, Backend};
use bm::utils::required_depth;
use primitive_types::U256;

use crate::{Composite, FixedVecRef, End, Intermediate, IntoVectorTree, IntoTree};

pub struct VariableVecRef<'a, T>(pub &'a [T], pub usize);
pub struct VariableVec<T>(pub Vec<T>, pub usize);

macro_rules! impl_packed {
    ( $t:ty, $len:expr ) => {
        impl<'a, DB> IntoTree<DB> for VariableVecRef<'a, $t> where
            DB: Backend<Intermediate=Intermediate, End=End>,
        {
            fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
                let target_depth = required_depth(self.1 * $len / 256);
                let len = self.0.len();

                let left = FixedVecRef(&self.0).into_vector_tree(db, Some(target_depth))?;
                let right = U256::from(len).into_tree(db)?;
                let key = db.intermediate_of(&left, &right);

                db.insert(key.clone(), (left, right))?;
                Ok(Value::Intermediate(key))
            }
        }
    }
}

impl_packed!(bool, 1);
impl_packed!(u8, 8);
impl_packed!(u16, 16);
impl_packed!(u32, 32);
impl_packed!(u64, 64);
impl_packed!(u128, 128);
impl_packed!(U256, 256);

impl<'a, DB, T: Composite> IntoTree<DB> for VariableVecRef<'a, T> where
    for<'b> FixedVecRef<'b, T>: IntoVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        let target_depth = required_depth(self.1);
        let len = self.0.len();

        let left = FixedVecRef(&self.0).into_vector_tree(db, Some(target_depth))?;
        let right = U256::from(len).into_tree(db)?;
        let key = db.intermediate_of(&left, &right);

        db.insert(key.clone(), (left, right))?;
        Ok(Value::Intermediate(key))
    }
}

impl<DB, T> IntoTree<DB> for VariableVec<T> where
    for<'a> VariableVecRef<'a, T>: IntoTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        VariableVecRef(&self.0, self.1).into_tree(db)
    }
}
