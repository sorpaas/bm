use merklist::{RawList, Value};
use sha2::Sha256;
use digest::Digest;
use core::num::NonZeroUsize;

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

type InMemory = merklist::InMemoryRawListDB<Sha256, Vec<u8>>;

#[test]
fn ssz_composite_fixed() {
    let ssz_value: (Vec<u8>, Vec<u8>) = (vec![2, 3, 4], vec![5, 6, 7]);
    let ssz_hash = ssz_value.hash::<Sha256Hasher>();

    let mut db = InMemory::default();
    let mut raw = RawList::<InMemory>::new_with_default(
        vec![0, 0, 0, 0, 0, 0, 0, 0,
             0, 0, 0, 0, 0, 0, 0, 0,
             0, 0, 0, 0, 0, 0, 0, 0,
             0, 0, 0, 0, 0, 0, 0, 0]
    );

    raw.set(&mut db, NonZeroUsize::new(2).unwrap(), Value::End(ssz_value.0.hash::<Sha256Hasher>()[..].to_vec()));
    raw.set(&mut db, NonZeroUsize::new(3).unwrap(), Value::End(ssz_value.1.hash::<Sha256Hasher>()[..].to_vec()));

    assert_eq!(&ssz_hash[..], raw.root().intermediate().unwrap().as_slice());
}
