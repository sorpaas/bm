use bm::{MerkleVec, MerkleTuple, MerklePackedVec, OwnedRoot};
use sha2::Sha256;
use digest::Digest;

use generic_array::GenericArray;
use hash_db::Hasher;
use primitive_types::H256;
use plain_hasher::PlainHasher;
use ssz::Hashable;
use typenum::{U1, U32};

/// Concrete `Hasher` impl for the Keccak-256 hash
pub struct Sha256Hasher;
impl Hasher for Sha256Hasher {
    type Out = H256;
    type StdHasher = PlainHasher;
    const LENGTH: usize = 32;

    fn hash(x: &[u8]) -> Self::Out {
        H256::from_slice(Sha256::digest(x).as_slice())
    }
}

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

impl From<H256> for VecValue {
    fn from(h: H256) -> VecValue {
        let mut raw = [0u8; 32];
        (&mut raw).copy_from_slice(&h[..]);
        VecValue(raw)
    }
}

impl From<GenericArray<u8, U32>> for VecValue {
    fn from(arr: GenericArray<u8, U32>) -> VecValue {
        let mut raw = [0u8; 32];
        (&mut raw).copy_from_slice(&arr[..]);
        VecValue(raw)
    }
}

impl Into<GenericArray<u8, U32>> for VecValue {
    fn into(self) -> GenericArray<u8, U32> {
        let mut arr: GenericArray<u8, U32> = Default::default();
        (&mut arr[..]).copy_from_slice(&self.0[..]);
        arr
    }
}

type InMemory = bm::InMemoryMerkleDB<Sha256, VecValue>;

#[test]
fn ssz_composite_fixed() {
    let ssz_value = (vec![2, 3, 4], vec![5, 6, 7], vec![8, 9, 10]);
    let ssz_hash = ssz_value.hash::<Sha256Hasher>();

    let mut db = InMemory::new_with_inherited_empty();
    let mut tuple = MerkleTuple::<OwnedRoot, InMemory>::create(&mut db, 3).unwrap();

    tuple.set(&mut db, 0, ssz_value.0.hash::<Sha256Hasher>().into()).unwrap();
    tuple.set(&mut db, 1, ssz_value.1.hash::<Sha256Hasher>().into()).unwrap();
    tuple.set(&mut db, 2, ssz_value.2.hash::<Sha256Hasher>().into()).unwrap();

    assert_eq!(&ssz_hash[..], tuple.root().intermediate().unwrap().as_slice());
}

#[test]
fn ssz_composite_variable() {
    let ssz_value = vec![vec![2, 3, 4], vec![5, 6, 7], vec![8, 9, 10]];
    let ssz_hash = ssz_value.hash::<Sha256Hasher>();

    let mut db = InMemory::new_with_inherited_empty();
    let mut vec = MerkleVec::<OwnedRoot, InMemory>::create(&mut db).unwrap();

    for v in ssz_value {
        vec.push(&mut db, v.hash::<Sha256Hasher>().into()).unwrap();
    }

    assert_eq!(&ssz_hash[..], vec.root().intermediate().unwrap().as_slice());
}

#[test]
fn ssz_composite_packed_variable() {
    let ssz_value = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                         18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33];
    let ssz_hash = ssz_value.hash::<Sha256Hasher>();

    let mut db = InMemory::new_with_inherited_empty();
    let mut vec = MerklePackedVec::<OwnedRoot, InMemory, GenericArray<u8, U1>, U32, U1>::create(&mut db).unwrap();
    for v in ssz_value {
        vec.push(&mut db, {
            let mut arr = GenericArray::<u8, U1>::default();
            arr[0] = v;
            arr
        }).unwrap();
    }

    assert_eq!(&ssz_hash[..], vec.root().intermediate().unwrap().as_slice());
}
