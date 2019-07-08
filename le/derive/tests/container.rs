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
    #[bm(vector, len = "4")]
    d: FixedVec<u64>,
    #[bm(list, max_len = "5")]
    e: VariableVec<u64>,
}
