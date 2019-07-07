use sha2::{Digest, Sha256};
use primitive_types::H256;
use bm_le::tree_root;
use bm_le_derive::IntoTree;

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

#[test]
fn test_basic() {
    assert_eq!(tree_root::<Sha256, _>(&BasicContainer { a: 1, b: 2, c: 3 }),
               h(&h(&chunk(&[0x01])[..], &chunk(&[0x02])[..])[..],
                 &h(&chunk(&[0x03])[..], &chunk(&[])[..])[..]));
}
