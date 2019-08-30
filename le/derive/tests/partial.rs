use bm_le::{FromTree, IntoTree, Partialable, DigestConstruct, PartialItem, PartialIndex, DanglingRaw};
use sha2::Sha256;
use bm::{InMemoryBackend, Tree};

#[derive(FromTree, IntoTree, Partialable)]
struct BasicContainer {
	a: u32,
	b: u64,
	c: u128,
}

#[derive(FromTree, IntoTree, Partialable)]
struct NestedContainer {
	d: u16,
	basic: BasicContainer,
}

#[test]
fn partial_test() {
	let mut db = InMemoryBackend::<DigestConstruct<Sha256>>::default();
	let mut full = NestedContainer {
		basic: BasicContainer {
			a: 1,
			b: 2,
			c: 3,
		},
		d: 4,
	};
	let root = full.into_tree(&mut db).unwrap();
	full.basic.c = 5;
	let new_root = full.into_tree(&mut db).unwrap();

	let mut raw = DanglingRaw::<DigestConstruct<Sha256>>::new(root);
	let mut partial = PartialNestedContainer::new(PartialIndex::root());
	assert_eq!(*partial.basic.c.get(&raw, &mut db).unwrap(), 3);
	partial.basic.c.set(5);

	partial.flush(&mut raw, &mut db).unwrap();
	assert_eq!(raw.root(), new_root);
}
