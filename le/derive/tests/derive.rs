use sha2::{Digest, Sha256};
use primitive_types::H256;
use bm::InMemoryBackend;
use bm_le::{FixedVec, VariableVec, IntoTree, FromTreeWithConfig, End, tree_root};
use bm_le_derive::{IntoTree, FromTree};

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

trait Config {
    fn d_len(&self) -> u64 { 4 }
    fn e_max_len(&self) -> u64 { 5 }
}

#[derive(IntoTree)]
struct BasicContainer {
    a: u32,
    b: u64,
    c: u128,
}

#[derive(IntoTree, FromTree, PartialEq, Eq, Debug)]
#[bm(config_trait = "Config")]
struct ConfigContainer {
    a: u64,
    b: u64,
    c: u64,
    #[bm(vector, len = "config.d_len() as usize")]
    d: FixedVec<u64>,
    e: u64,
    #[bm(list, max_len = "config.e_max_len() as usize")]
    f: VariableVec<u64>,
}

#[test]
fn test_basic() {
    assert_eq!(tree_root::<Sha256, _>(&BasicContainer { a: 1, b: 2, c: 3 }),
               h(&h(&chunk(&[0x01])[..], &chunk(&[0x02])[..])[..],
                 &h(&chunk(&[0x03])[..], &chunk(&[])[..])[..]));
}

struct TestConfig;

impl Config for TestConfig { }

#[test]
fn test_config() {
    let mut db = InMemoryBackend::<Sha256, End>::new_with_inherited_empty();
    let container = ConfigContainer {
        a: 1,
        b: 2,
        c: 3,
        d: FixedVec(vec![4, 5, 6, 7]),
        e: 8,
        f: VariableVec(vec![9, 10], Some(5)),
    };
    let actual = container.into_tree(&mut db).unwrap();
    let decoded = ConfigContainer::from_tree_with_config(&actual, &db, &TestConfig).unwrap();
    assert_eq!(container, decoded);
}
