# bm

![Crates.io](https://img.shields.io/crates/v/bm.svg)
![Docs](https://docs.rs/bm/badge.svg)

Binary merkle tree implementation with garbage collection support. The crate contains:

* `MerkleRaw`: Raw binary merkle tree that allows directly operating
  on generalized merkle index.
* `MerkleVec`: Variable-sized vector list.
* `MerkleTuple`: Fixed-sized tuple list.
* `MerklePackedVec`: Packed variable-sized vector list.
* `MerklePackedTuple`: Packed fixed-sized tuple list.

The crate also contains an in-memory backend with garbage collection support.

## Basic Usage

```rust
use sha2::Sha256;
use digest::Digest;
use bm::MerkleVec;

type InMemory = bm::InMemoryMerkleDB<Sha256, Vec<u8>>;

let mut db = InMemory::default();
let mut vec = MerkleVec::<InMemory>::create(&mut db);

for i in 0..100 {
    vec.push(&mut db, vec![i as u8]);
}

vec.drop(&mut db);
```
