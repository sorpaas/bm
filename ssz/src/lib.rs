use typenum::U32;
use generic_array::GenericArray;

#[derive(Clone)]
pub struct End([u8; 32]);

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
