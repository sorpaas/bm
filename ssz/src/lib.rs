use typenum::U32;
use generic_array::GenericArray;
use primitive_types::H256;
use digest::Digest;
use bm::{Backend, NoopBackend, Error};
use bm::serialize::Serialize;

mod basic;
mod fixed;

#[derive(Clone)]
pub struct End(pub [u8; 32]);

impl Default for End {
    fn default() -> Self {
        Self([0; 32])
    }
}

impl AsRef<[u8]> for End {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

pub type Intermediate = GenericArray<u8, U32>;

pub trait Composite { }

pub struct Serial<'a, T>(pub &'a T);

pub fn serialize<'a, T, DB>(value: &'a T, db: &mut DB) -> Result<H256, Error<DB::Error>> where
    Serial<'a, T>: Serialize<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    Serial(value).serialize(db).map(|ret| H256::from_slice(ret.as_ref()))
}

pub fn serialize_noop<'a, D, T>(value: &'a T) -> H256 where
    D: Digest<OutputSize=U32>,
    Serial<'a, T>: Serialize<NoopBackend<D, End>>,
{
    serialize(value, &mut NoopBackend::new_with_inherited_empty())
        .expect("Noop backend never fails in set; qed")
}
