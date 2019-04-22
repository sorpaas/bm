# bm

Binary merkle tree implementation with garbage collection support.

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
