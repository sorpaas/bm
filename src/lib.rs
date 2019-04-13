use digest::Digest;
use generic_array::GenericArray;

use std::collections::HashMap;
use core::mem;

pub struct List<D: Digest> {
    db: HashMap<GenericArray<u8, D::OutputSize>, (GenericArray<u8, D::OutputSize>, GenericArray<u8, D::OutputSize>)>,
    default_value: GenericArray<u8, D::OutputSize>,
    last_default_root: GenericArray<u8, D::OutputSize>,
    root: GenericArray<u8, D::OutputSize>,
    len: usize,
}

impl<D: Digest> List<D> {
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
            len: 2,
            last_default_root: default_value.clone(),
            db,
            root,
            default_value,
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
        self.len = self.len * 2;
    }

    pub fn shrink(&mut self) {
        debug_assert!(self.len().is_power_of_two() && self.len() >= 2);

        if self.len() <= 2 {
            return
        }

        let new_last_default_root = self.db.get(&self.last_default_root)
            .expect("Last default root must exist; qed").0.clone();
        let new_root = self.db.get(&self.root)
            .expect("Root must exist; qed").0.clone();

        self.len = self.len / 2;
        self.root = new_root;
        self.last_default_root = new_last_default_root;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn depth(&self) -> usize {
        debug_assert!(self.len().is_power_of_two() && self.len() != 0);

        mem::size_of::<usize>() * 8 - self.len().leading_zeros() as usize - 1
    }

    pub fn get(&self, index: usize) -> Option<GenericArray<u8, D::OutputSize>> {
        debug_assert!(self.len().is_power_of_two());

        if index >= self.len() {
            return None
        }
        let depth = self.depth();

        let mut current = self.root.clone();
        for d in 0..depth {
            let sel = (index & (0b1 << (depth - 1 - d))) >> (depth - 1 - d);
            current = self.db.get(&current).map(|values| {
                if sel == 0 {
                    values.0.clone()
                } else {
                    values.1.clone()
                }
            }).expect("Any depth level key must exists; qed");
        }

        Some(current)
    }

    pub fn set(&mut self, index: usize, set: GenericArray<u8, D::OutputSize>) {
        debug_assert!(self.len().is_power_of_two());

        if index >= self.len() {
            return
        }
        let depth = self.depth();

        let mut current = self.root.clone();
        let mut values = Vec::new();
        for d in 0..depth {
            let sel = (index & (0b1 << (depth - 1 - d))) >> (depth - 1 - d);
            println!("index: {}, d: {}, sel: {}", index, d, sel);
            let value = self.db.get(&current).expect("Any depth level key must exists; qed");
            values.push((sel, value.clone()));
            current = if sel == 0 {
                value.0.clone()
            } else {
                value.1.clone()
            };
        }

        let mut update = set;
        while !values.is_empty() {
            let (sel, mut value) = values.pop().expect("values checked not to be empty; qed");
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
        let mut list = List::<Sha256>::new();
        list.extend();
        assert_eq!(list.len(), 4);

        for i in 0..list.len() {
            assert_eq!(list.get(i), Some(Default::default()));
        }
    }

    #[test]
    fn test_set_and_shrink() {
        let mut list = List::<Sha256>::new();
        list.extend();
        assert_eq!(list.len(), 4);
        for i in 0..list.len() {
            let mut arr = [0u8; 32];
            arr[0] = i as u8;
            list.set(i, arr.into());
        }
        for i in 0..list.len() {
            let val = list.get(i).unwrap();
            assert_eq!(val[0], i as u8);
        }
        list.shrink();
        assert_eq!(list.len(), 2);
        for i in 0..list.len() {
            let val = list.get(i).unwrap();
            println!("val: {:?}", val);
            assert_eq!(val[0], i as u8);
        }
    }
}
