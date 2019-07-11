use bm_le::MaxVec;
use bm_le_derive::{FromTree, IntoTree};
use generic_array::GenericArray;

pub trait Config {
    fn d_len(&self) -> u64 { 4 }
    fn e_max_len(&self) -> u64 { 5 }
}

#[derive(IntoTree, FromTree)]
pub struct Container {
    a: u32,
    b: u64,
    c: u128,
    d: GenericArray<u64, typenum::U4>,
    e: MaxVec<u64, typenum::U5>,
}
