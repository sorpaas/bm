use bm_le::{FixedVec, VariableVec};
use bm_le_derive::{FromTree, IntoTree};

pub trait Config {
    fn d_len(&self) -> u64 { 4 }
    fn e_max_len(&self) -> u64 { 5 }
}

#[derive(IntoTree, FromTree)]
#[bm(config_trait = "Config")]
pub struct Container {
    a: u32,
    b: u64,
    c: u128,
    d: FixedVec<u64, typenum::U4>,
    e: VariableVec<u64, typenum::U5>,
}
