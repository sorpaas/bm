use crate::traits::{Backend, EndOf, Value, ValueOf, RootStatus, Owned, Dangling, Leak, Error, Tree, Sequence};
use crate::raw::Raw;
use crate::index::Index;

const ROOT_INDEX: Index = Index::root();
const EXTEND_INDEX: Index = Index::root().left();
const EMPTY_INDEX: Index = Index::root().right();

/// `Vector` with owned root.
pub type OwnedVector<DB> = Vector<Owned, DB>;

/// `Vector` with dangling root.
pub type DanglingVector<DB> = Vector<Dangling, DB>;

/// Binary merkle tuple.
pub struct Vector<R: RootStatus, DB: Backend> {
    raw: Raw<R, DB>,
    max_len: Option<usize>,
    len: usize,
}

impl<R: RootStatus, DB: Backend> Vector<R, DB> {
    fn raw_index(&self, i: usize) -> Result<Index, Error<DB::Error>> {
        Index::from_one(self.current_max_len() + i).ok_or(Error::InvalidParameter)
    }

    fn extend(&mut self, db: &mut DB) -> Result<(), Error<DB::Error>> {
        let root = self.root();
        let mut new_raw = Raw::default();
        let empty = db.empty_at(self.depth())?;
        new_raw.set(db, EXTEND_INDEX, root)?;
        new_raw.set(db, EMPTY_INDEX, empty)?;
        self.raw.set(db, ROOT_INDEX, Value::End(Default::default()))?;
        self.raw = new_raw;
        Ok(())
    }

    fn shrink(&mut self, db: &mut DB) -> Result<(), Error<DB::Error>> {
        match self.raw.get(db, EXTEND_INDEX)? {
            Some(extended_value) => { self.raw.set(db, ROOT_INDEX, extended_value)?; },
            None => { self.raw.set(db, ROOT_INDEX, Value::End(Default::default()))?; },
        }
        Ok(())
    }

    /// Current maximum length of the vector.
    pub fn current_max_len(&self) -> usize {
        self.max_len.unwrap_or({
            let mut max_len = 1;
            while max_len < self.len {
                max_len *= 2;
            }
            max_len
        })
    }

    /// Overall maximum length of the vector.
    pub fn max_len(&self) -> Option<usize> {
        self.max_len
    }

    /// Depth of the vector.
    pub fn depth(&self) -> usize {
        let mut max_len = 1;
        let mut depth = 0;
        while max_len < self.current_max_len() {
            max_len *= 2;
            depth += 1;
        }
        depth
    }

    /// Get value at index.
    pub fn get(&self, db: &DB, index: usize) -> Result<EndOf<DB>, Error<DB::Error>> {
        if index >= self.len() {
            return Err(Error::AccessOverflowed)
        }

        let raw_index = self.raw_index(index)?;
        self.raw.get(db, raw_index)?.ok_or(Error::CorruptedDatabase)?
            .end().ok_or(Error::CorruptedDatabase)
    }

    /// Set value at index.
    pub fn set(&mut self, db: &mut DB, index: usize, value: EndOf<DB>) -> Result<(), Error<DB::Error>> {
        if index >= self.len() {
            return Err(Error::AccessOverflowed)
        }

        let raw_index = self.raw_index(index)?;
        self.raw.set(db, raw_index, Value::End(value))?;
        Ok(())
    }

    /// Push a new value to the vector.
    pub fn push(&mut self, db: &mut DB, value: EndOf<DB>) -> Result<(), Error<DB::Error>> {
        let old_len = self.len();
        if old_len == self.current_max_len() {
            if self.max_len.is_some() {
                return Err(Error::AccessOverflowed)
            } else {
                self.extend(db)?;
            }
        }
        let len = old_len + 1;
        let index = old_len;
        self.len = len;

        let raw_index = self.raw_index(index)?;
        self.raw.set(db, raw_index, Value::End(value))?;
        Ok(())
    }

    /// Pop a value from the vector.
    pub fn pop(&mut self, db: &mut DB) -> Result<Option<EndOf<DB>>, Error<DB::Error>> {
        let old_len = self.len();
        if old_len == 0 {
            return Ok(None)
        }

        let len = old_len - 1;
        let index = old_len - 1;
        let raw_index = self.raw_index(index)?;
        let value = match self.raw.get(db, raw_index)? {
            Some(value) => value.end().ok_or(Error::CorruptedDatabase)?,
            None => return Err(Error::CorruptedDatabase),
        };

        let mut empty_depth_to_bottom = 0;
        let mut replace_index = raw_index;
        loop {
            if let Some(parent) = replace_index.parent() {
                if parent.left() == replace_index {
                    replace_index = parent;
                    empty_depth_to_bottom += 1;
                } else {
                    break
                }
            } else {
                break
            }
        }
        let empty = db.empty_at(empty_depth_to_bottom)?;
        self.raw.set(db, replace_index, empty)?;

        if len <= self.current_max_len() / 2 {
            if self.max_len.is_none() {
                self.shrink(db)?;
            }
        }
        self.len = len;
        Ok(Some(value))
    }

    /// Get the length of the tuple.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Create a tuple from raw merkle tree.
    pub fn from_raw(raw: Raw<R, DB>, len: usize, max_len: Option<usize>) -> Self {
        Self { raw, len, max_len }
    }
}

impl<R: RootStatus, DB: Backend> Tree for Vector<R, DB> {
    type RootStatus = R;
    type Backend = DB;

    fn root(&self) -> ValueOf<DB> {
        self.raw.root()
    }

    fn drop(self, db: &mut DB) -> Result<(), Error<DB::Error>> {
        self.raw.drop(db)?;
        Ok(())
    }

    fn into_raw(self) -> Raw<R, DB> {
        self.raw
    }
}

impl<R: RootStatus, DB: Backend> Sequence for Vector<R, DB> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<R: RootStatus, DB: Backend> Leak for Vector<R, DB> {
    type Metadata = (ValueOf<DB>, usize, Option<usize>);

    fn metadata(&self) -> Self::Metadata {
        let len = self.len();
        let max_len = self.max_len;
        (self.raw.metadata(), len, max_len)
    }

    fn from_leaked((raw_root, len, max_len): Self::Metadata) -> Self {
        Self {
            raw: Raw::from_leaked(raw_root),
            len,
            max_len,
        }
    }
}

impl<DB: Backend> Vector<Owned, DB> {
    /// Create a new tuple.
    pub fn create(db: &mut DB, len: usize, max_len: Option<usize>) -> Result<Self, Error<DB::Error>> {
        if let Some(max_len) = max_len {
            if len < max_len || max_len == 0 {
                return Err(Error::InvalidParameter)
            }
        }

        let mut raw = Raw::<Owned, DB>::default();

        let target_len = max_len.unwrap_or(len);
        let mut current_max_len = 1;
        let mut depth = 0;
        while current_max_len < target_len {
            current_max_len *= 2;
            depth += 1;
        }

        let empty = db.empty_at(depth)?;
        raw.set(db, ROOT_INDEX, empty)?;

        Ok(Self {
            raw,
            len,
            max_len,
        })
    }
}
