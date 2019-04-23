use bm::{MerkleVec, MerkleTuple};
use sha2::Sha256;
use digest::Digest;

use hash_db::Hasher;
use primitive_types::H256;
use plain_hasher::PlainHasher;
use ssz::Hashable;

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
struct VecValue(Vec<u8>);

impl AsRef<[u8]> for VecValue {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<usize> for VecValue {
    fn from(value: usize) -> Self {
        let mut bytes: Vec<u8> = (&(value as u64).to_le_bytes()[..]).into();
        while bytes.len() != 32 {
            bytes.push(0);
        }
        VecValue(bytes)
    }
}

impl Into<usize> for VecValue {
    fn into(self) -> usize {
        let mut raw = [0u8; 8];
        (&mut raw).copy_from_slice(&self.0[0..8]);
        u64::from_le_bytes(raw) as usize
    }
}

impl Default for VecValue {
    fn default() -> Self {
        VecValue(vec![0, 0, 0, 0, 0, 0, 0, 0,
                      0, 0, 0, 0, 0, 0, 0, 0,
                      0, 0, 0, 0, 0, 0, 0, 0,
                      0, 0, 0, 0, 0, 0, 0, 0])
    }
}

type InMemory = bm::InMemoryMerkleDB<Sha256, VecValue>;

#[test]
fn ssz_composite_fixed() {
    let ssz_value = (vec![2, 3, 4], vec![5, 6, 7], vec![8, 9, 10]);
    let ssz_hash = ssz_value.hash::<Sha256Hasher>();

    let mut db = InMemory::default();
    let mut tuple = MerkleTuple::<InMemory>::create(&mut db, 3);

    tuple.set(&mut db, 0, VecValue(ssz_value.0.hash::<Sha256Hasher>()[..].to_vec()));
    tuple.set(&mut db, 1, VecValue(ssz_value.1.hash::<Sha256Hasher>()[..].to_vec()));
    tuple.set(&mut db, 2, VecValue(ssz_value.2.hash::<Sha256Hasher>()[..].to_vec()));

    assert_eq!(&ssz_hash[..], tuple.root().intermediate().unwrap().as_slice());
}

#[test]
fn ssz_composite_variable() {
    let ssz_value = vec![vec![2, 3, 4], vec![5, 6, 7], vec![8, 9, 10]];
    let ssz_hash = ssz_value.hash::<Sha256Hasher>();

    let mut db = InMemory::default();
    let mut vec = MerkleVec::<InMemory>::create(&mut db);

    for v in ssz_value {
        vec.push(&mut db, VecValue(v.hash::<Sha256Hasher>()[..].to_vec()));
    }

    assert_eq!(&ssz_hash[..], vec.root().intermediate().unwrap().as_slice());
}
