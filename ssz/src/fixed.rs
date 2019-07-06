use bm::{ValueOf, Backend, Error, Value};
use bm::utils::vector_tree;
use primitive_types::{U256, H256};

use crate::{IntoTree, Intermediate, End, Composite};

pub trait IntoVectorTree<DB: Backend<Intermediate=Intermediate, End=End>> {
    fn into_vector_tree(
        &self,
        db: &mut DB,
        at_depth: Option<usize>
    ) -> Result<ValueOf<DB>, Error<DB::Error>>;
}

pub struct FixedVecRef<'a, T>(pub &'a [T]);
pub struct FixedVec<T>(pub Vec<T>);

macro_rules! impl_builtin_fixed_uint_vector {
    ( $( $t:ty ),* ) => { $(
        impl<'a, DB> IntoVectorTree<DB> for FixedVecRef<'a, $t> where
            DB: Backend<Intermediate=Intermediate, End=End>
        {
            fn into_vector_tree(
                &self,
                db: &mut DB,
                at_depth: Option<usize>
            ) -> Result<ValueOf<DB>, Error<DB::Error>> {
                let mut chunks: Vec<Vec<u8>> = Vec::new();

                for value in self.0 {
                    if chunks.last().map(|v| v.len() == 32).unwrap_or(true) {
                        chunks.push(Vec::new());
                    }

                    let current = chunks.last_mut().expect("chunks must have at least one item; qed");
                    current.append(&mut value.to_le_bytes().into_iter().cloned().collect::<Vec<u8>>());
                }

                if let Some(last) = chunks.last_mut() {
                    while last.len() < 32 {
                        last.push(0u8);
                    }
                }

                vector_tree(&chunks.into_iter().map(|c| {
                    let mut ret = End::default();
                    ret.0.copy_from_slice(&c);
                    Value::End(ret)
                }).collect::<Vec<_>>(), db, at_depth)
            }
        }
    )* }
}

impl_builtin_fixed_uint_vector!(u8, u16, u32, u64, u128);

impl<'a, DB> IntoVectorTree<DB> for FixedVecRef<'a, U256> where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn into_vector_tree(
        &self,
        db: &mut DB,
        at_depth: Option<usize>
    ) -> Result<ValueOf<DB>, Error<DB::Error>> {
        vector_tree(&self.0.iter().map(|uint| {
            let mut ret = End::default();
            uint.to_little_endian(&mut ret.0);
            Value::End(ret)
        }).collect::<Vec<_>>(), db, at_depth)
    }
}

impl<'a, DB> IntoVectorTree<DB> for FixedVecRef<'a, bool> where
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_vector_tree(
        &self,
        db: &mut DB,
        at_depth: Option<usize>
    ) -> Result<ValueOf<DB>, Error<DB::Error>> {
        let mut bytes = Vec::new();
        bytes.resize((self.0.len() + 7) / 8, 0u8);

        for i in 0..self.0.len() {
            bytes[i / 8] |= (self.0[i] as u8) << (i % 8);
        }

        FixedVecRef(&bytes).into_vector_tree(db, at_depth)
    }
}

impl<'a, DB, T: Composite> IntoVectorTree<DB> for FixedVecRef<'a, T> where
    T: IntoTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_vector_tree(
        &self,
        db: &mut DB,
        at_depth: Option<usize>
    ) -> Result<ValueOf<DB>, Error<DB::Error>> {
        vector_tree(&self.0.iter().map(|value| {
            value.into_tree(db)
        }).collect::<Result<Vec<_>, _>>()?, db, at_depth)
    }
}

impl<'a, DB, T> IntoTree<DB> for FixedVecRef<'a, T> where
    Self: IntoVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        self.into_vector_tree(db, None)
    }
}

impl<DB, T> IntoVectorTree<DB> for FixedVec<T> where
    for<'a> FixedVecRef<'a, T>: IntoVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn into_vector_tree(
        &self,
        db: &mut DB,
        at_depth: Option<usize>
    ) -> Result<ValueOf<DB>, Error<DB::Error>> {
        FixedVecRef(&self.0).into_vector_tree(db, at_depth)
    }
}

impl<DB, T> IntoTree<DB> for FixedVec<T> where
    Self: IntoVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        self.into_vector_tree(db, None)
    }
}

impl<DB> IntoTree<DB> for H256 where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        FixedVecRef(&self.0.as_ref()).into_tree(db)
    }
}
