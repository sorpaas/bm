use sha2::{Digest, Sha256};
use primitive_types::H256;
use bm::InMemoryBackend;
use bm_le::{IntoTree, FromTree, End, MaxVec, tree_root};
use generic_array::GenericArray;

fn chunk(data: &[u8]) -> H256 {
    let mut ret = [0; 32];
    ret[..data.len()].copy_from_slice(data);

    H256::from(ret)
}

fn h(a: &[u8], b: &[u8]) -> H256 {
    let mut hash = Sha256::new();
    hash.input(a);
    hash.input(b);
    H256::from_slice(hash.result().as_slice())
}

#[derive(IntoTree)]
struct BasicContainer {
    a: u32,
    b: u64,
    c: u128,
}

#[derive(IntoTree, FromTree, PartialEq, Eq, Debug)]
struct ConfigContainer {
    a: u64,
    b: u64,
    c: u64,
    #[bm(compact)]
    d: GenericArray<u64, typenum::U4>,
    e: u64,
    #[bm(compact)]
    f: MaxVec<u64, typenum::U5>,
}

#[test]
fn test_basic() {
    assert_eq!(tree_root::<Sha256, _>(&BasicContainer { a: 1, b: 2, c: 3 }),
               h(&h(&chunk(&[0x01])[..], &chunk(&[0x02])[..])[..],
                 &h(&chunk(&[0x03])[..], &chunk(&[])[..])[..]));
}

#[test]
fn test_config() {
    let mut db = InMemoryBackend::<Sha256, End>::new_with_inherited_empty();
    let container = ConfigContainer {
        a: 1,
        b: 2,
        c: 3,
        d: GenericArray::from([4, 5, 6, 7]),
        e: 8,
        f: MaxVec::from(vec![9, 10]),
    };
    let actual = container.into_tree(&mut db).unwrap();
    let decoded = ConfigContainer::from_tree(&actual, &db).unwrap();
    assert_eq!(container, decoded);
}
