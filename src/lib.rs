use digest::Digest;
use generic_array::GenericArray;

use std::collections::HashMap;
use core::num::NonZeroUsize;

pub enum Value<D: Digest> {
    Intermediate(GenericArray<u8, D::OutputSize>),
    End(Vec<u8>),
}

pub struct RawList<D: Digest> {
    db: HashMap<GenericArray<u8, D::OutputSize>, (GenericArray<u8, D::OutputSize>, GenericArray<u8, D::OutputSize>)>,
    default_value: GenericArray<u8, D::OutputSize>,
    last_default_root: GenericArray<u8, D::OutputSize>,
    root: GenericArray<u8, D::OutputSize>,
    depth: u32,
}

fn selection_at(index: NonZeroUsize, depth: u32) -> Option<usize> {
    let mut index = index.get();
    if index < 2_usize.pow(depth) {
        return None
    }

    while index > 2_usize.pow(depth + 1) {
        index = index / 2;
    }
    Some(index % 2)
}

impl<D: Digest> RawList<D> {
    pub fn new_with_default(default_value: GenericArray<u8, D::OutputSize>) -> Self {
        let root = {
            let mut digest = D::new();
            digest.input(&default_value[..]);
            digest.input(&default_value[..]);
            digest.result()
        };

        let mut db = HashMap::new();
        db.insert(root.clone(), (default_value.clone(), default_value.clone()));

        Self {
            depth: 1,
            last_default_root: default_value.clone(),
            default_value,
            db,
            root,
        }
    }

    pub fn new() -> Self {
        Self::new_with_default(Default::default())
    }

    pub fn extend(&mut self) {
        let new_last_default_root = {
            let mut digest = D::new();
            digest.input(&self.last_default_root[..]);
            digest.input(&self.last_default_root[..]);
            digest.result()
        };
        self.db.insert(new_last_default_root.clone(), (self.last_default_root.clone(), self.last_default_root.clone()));

        let new_root = {
            let mut digest = D::new();
            digest.input(&self.root[..]);
            digest.input(&new_last_default_root[..]);
            digest.result()
        };
        self.db.insert(new_root.clone(), (self.root.clone(), new_last_default_root.clone()));

        self.last_default_root = new_last_default_root;
        self.root = new_root;
        self.depth += 1;
    }

    pub fn shrink(&mut self) {
        if self.depth < 2 {
            return
        }

        let new_last_default_root = self.db.get(&self.last_default_root)
            .expect("Last default root must exist; qed").0.clone();
        let new_root = self.db.get(&self.root)
            .expect("Root must exist; qed").0.clone();

        self.depth -= 1;
        self.root = new_root;
        self.last_default_root = new_last_default_root;
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn get(&self, index: NonZeroUsize) -> Option<GenericArray<u8, D::OutputSize>> {
        let mut current = self.root.clone();
        let mut depth = 1;
        loop {
            let sel = match selection_at(index, depth) {
                Some(sel) => sel,
                None => break,
            };
            current = match self.db.get(&current) {
                Some(value) => {
                    if sel == 0 {
                        value.0.clone()
                    } else {
                        value.1.clone()
                    }
                },
                None => return None,
            };
            depth += 1;
        }

        Some(current)
    }

    pub fn set(&mut self, index: NonZeroUsize, set: GenericArray<u8, D::OutputSize>) {
        let mut current = self.root.clone();
        let mut depth = 1;
        let mut values = Vec::new();
        loop {
            let sel = match selection_at(index, depth) {
                Some(sel) => sel,
                None => break,
            };
            let value = match self.db.get(&current) {
                Some(value) => value.clone(),
                None => (self.default_value.clone(), self.default_value.clone()),
            };
            values.push((sel, value.clone()));
            current = if sel == 0 {
                value.0.clone()
            } else {
                value.1.clone()
            };
            depth += 1;
        }

        let mut update = set;
        loop {
            let (sel, mut value) = match values.pop() {
                Some(v) => v,
                None => break,
            };

            if sel == 0 {
                value.0 = update;
            } else {
                value.1 = update;
            }
            update = {
                let mut digest = D::new();
                digest.input(&value.0[..]);
                digest.input(&value.1[..]);
                digest.result()
            };
            self.db.insert(update.clone(), value);
        }

        self.root = update;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Sha256;

    #[test]
    fn test_extend_and_get() {
        let mut list = RawList::<Sha256>::new();
        list.extend();
        assert_eq!(list.depth(), 2);

        for i in 4..8 {
            assert_eq!(list.get(NonZeroUsize::new(i).unwrap()), Some(Default::default()));
        }
    }

    #[test]
    fn test_set_and_shrink() {
        let mut list = RawList::<Sha256>::new();
        list.extend();
        assert_eq!(list.depth(), 2);
        for i in 4..8 {
            let mut arr = [0u8; 32];
            arr[0] = i as u8;
            list.set(NonZeroUsize::new(i).unwrap(), arr.into());
        }
        for i in 4..8 {
            let val = list.get(NonZeroUsize::new(i).unwrap()).unwrap();
            assert_eq!(val[0], i as u8);
        }
        list.shrink();
        assert_eq!(list.depth(), 1);
        for i in 2..3 {
            let val = list.get(NonZeroUsize::new(i).unwrap()).unwrap();
            assert_eq!(val[0], (i + 2) as u8);
        }
    }
}
