use bm::{OwnedList, ProvingBackend, Sequence, Value};
use sha2::Sha256;

#[derive(Clone, PartialEq, Eq, Debug)]
struct VecValue([u8; 32]);

impl AsRef<[u8]> for VecValue {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<usize> for VecValue {
    fn from(value: usize) -> Self {
        let mut bytes = [0u8; 32];
        (&mut bytes[0..8]).copy_from_slice(&(value as u64).to_le_bytes()[..]);
        VecValue(bytes)
    }
}

impl Into<usize> for VecValue {
    fn into(self) -> usize {
        let mut raw = [0u8; 8];
        (&mut raw[..]).copy_from_slice(&self.0[0..8]);
        u64::from_le_bytes(raw) as usize
    }
}

impl Default for VecValue {
    fn default() -> Self {
        VecValue([0, 0, 0, 0, 0, 0, 0, 0,
                  0, 0, 0, 0, 0, 0, 0, 0,
                  0, 0, 0, 0, 0, 0, 0, 0,
                  0, 0, 0, 0, 0, 0, 0, 0])
    }
}

type InMemory = bm::InMemoryBackend<bm::DigestConstruct<Sha256, VecValue>>;

#[test]
fn basic_proving_vec() {
    let mut db = InMemory::new_with_inherited_empty();
    let mut proving = ProvingBackend::new(&mut db);
    let mut vec = OwnedList::create(&mut proving, None).unwrap();

    for i in 0..100 {
        assert_eq!(vec.len(), i);
        vec.push(&mut proving, Value::End(i.into())).unwrap();
    }
    proving.reset();

    vec.get(&mut proving, 5usize.into()).unwrap();
    vec.get(&mut proving, 7usize.into()).unwrap();
    let vec_hash = vec.deconstruct(&mut proving).unwrap();
    let proofs = proving.reset();

    let mut proved = InMemory::new_with_inherited_empty();
    proved.populate(proofs);
    let proved_vec = OwnedList::reconstruct(vec_hash, &mut proved, None).unwrap();
    assert_eq!(proved_vec.get(&mut proved, 5usize.into()).unwrap(), Value::End(5usize.into()));
    assert_eq!(proved_vec.get(&mut proved, 7usize.into()).unwrap(), Value::End(7usize.into()));
}
