use bm::{Index, Error, ReadBackend, RootStatus, Raw, DanglingList, Tree, WriteBackend};
use primitive_types::{U256, H256};
use core::mem;
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as Map;
#[cfg(feature = "std")]
use std::collections::HashMap as Map;
use crate::{FromTree, IntoTree, CompatibleConstruct};

/// Partial index for le binary tree.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PartialIndex {
    parent: Option<Box<PartialIndex>>,
    sub: PartialSubIndex,
}

/// Partial sub-index for le binary tree.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum PartialSubIndex {
    /// Static index.
    Raw(Index),
    /// Index that is dynamic to a length.
    List(usize),
    /// Index from a vector.
    Vector(usize, usize),
}

impl PartialIndex {
    /// Partial index from root.
    pub fn root() -> Self {
        Self {
            parent: None,
            sub: PartialSubIndex::Raw(Index::root()),
        }
    }

    /// Partial index to a raw item.
    pub fn raw(&self, index: Index) -> Self {
        Self {
            parent: Some(Box::new(self.clone())),
            sub: PartialSubIndex::Raw(index),
        }
    }

    /// Partial index to a vector.
    pub fn vector(&self, index: usize, len: usize) -> Self {
        Self {
            parent: Some(Box::new(self.clone())),
            sub: PartialSubIndex::Vector(index, len),
        }
    }

    /// Partial index to a list.
    pub fn list(&self, index: usize) -> Self {
        Self {
            parent: Some(Box::new(self.clone())),
            sub: PartialSubIndex::List(index),
        }
    }

    /// Resolve the index.
    pub fn resolve<R: RootStatus, DB: ReadBackend>(
        &self,
        raw: &Raw<R, DB::Construct>,
        db: &mut DB
    ) -> Result<Index, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let parent = match &self.parent {
            Some(p) => p.resolve(raw, db)?,
            None => Index::root(),
        };

        let (index, len) = match self.sub {
            PartialSubIndex::Raw(raw) => return Ok(parent.sub(raw)),
            PartialSubIndex::Vector(index, len) => (index, len),
            PartialSubIndex::List(index) => {
                let len_root = raw.get(db, parent.right())?.ok_or(Error::CorruptedDatabase)?;
                let len = U256::from_tree(&len_root, db)?;

                if len > U256::from(usize::max_value()) {
                    return Err(Error::CorruptedDatabase)
                } else {
                    (index, len.as_usize())
                }
            },
        };

        let mut max_len = 1;
        let mut depth = 0;
        while max_len < len {
            max_len *= 2;
            depth += 1;
        }
        let sub = Index::from_one((1 << depth) + index).expect("
          result is greater or equal to 1;
          Index::from_one always return Some; qed");

        Ok(parent.sub(sub))
    }
}

/// Basic partial values.
pub struct PartialValue<T> {
    index: PartialIndex,
    value: Option<T>,
}

impl<T: FromTree> PartialValue<T> {
    /// Fetch the partial value from the database.
    pub fn fetch<R: RootStatus, DB: ReadBackend>(
        &mut self,
        raw: &Raw<R, DB::Construct>,
        db: &mut DB,
    ) -> Result<(), Error<DB::Error>> where
        DB::Construct: CompatibleConstruct
    {
        let index = self.index.resolve(raw, db)?;
        let index_root = raw.get(db, index)?.ok_or(Error::CorruptedDatabase)?;
        let value = T::from_tree(&index_root, db)?;

        self.value = Some(value);
        Ok(())
    }

    /// Get a reference to the fetched partial value.
    pub fn get<R: RootStatus, DB: ReadBackend>(
        &mut self,
        raw: &Raw<R, DB::Construct>,
        db: &mut DB,
    ) -> Result<&T, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct
    {
        if self.value.is_none() {
            self.fetch(raw, db)?;
        }

        Ok(self.value.as_ref().expect("value is checked to be some or set before; qed"))
    }

    /// Set the partial value.
    pub fn set(&mut self, value: T) {
        self.value = Some(value);
    }
}

impl<T: IntoTree> PartialItem for PartialValue<T> {
    fn new(index: PartialIndex) -> Self {
        Self {
            index,
            value: None
        }
    }

    fn flush<R: RootStatus, DB: WriteBackend>(
        &mut self,
        raw: &mut Raw<R, DB::Construct>,
        db: &mut DB,
    ) -> Result<(), Error<DB::Error>> where
        DB::Construct: CompatibleConstruct
    {
        if let Some(value) = self.value.take() {
            let index = self.index.resolve(raw, db)?;
            let value_root = value.into_tree(db)?;

            raw.set(db, index, value_root)?;
        }

        Ok(())
    }
}

/// Partial item for Vec.
pub struct PartialVec<T: Partialable> {
    index: PartialIndex,
    values: Map<usize, T::Value>,
    pushed: Vec<T>,
}

impl<T: Partialable> PartialVec<T> {
    /// Access a value at given position.
    pub fn at(&mut self, index: usize) -> &mut T::Value {
        self.values.entry(index).or_insert(PartialItem::new(PartialIndex {
            parent: Some(Box::new(self.index.clone())),
            sub: PartialSubIndex::List(index),
        }))
    }

    /// Push a value at given position.
    pub fn push(&mut self, value: T) {
        self.pushed.push(value);
    }
}

impl<T: Partialable + IntoTree> PartialItem for PartialVec<T> {
    fn new(index: PartialIndex) -> Self {
        Self {
            index,
            values: Default::default(),
            pushed: Default::default(),
        }
    }

    fn flush<R: RootStatus, DB: WriteBackend>(
        &mut self,
        raw: &mut Raw<R, DB::Construct>,
        db: &mut DB,
    ) -> Result<(), Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let mut values = Map::default();
        mem::swap(&mut values, &mut self.values);

        for (_, mut value) in values {
            value.flush(raw, db)?;
        }

        let mut pushed = Vec::default();
        mem::swap(&mut pushed, &mut self.pushed);
        let mut list = DanglingList::reconstruct(raw.root(), db, None)?;
        for value in pushed {
            let value_root = value.into_tree(db)?;
            list.push(db, value_root)?;
        }

        Ok(())
    }
}

/// Partial item.
pub trait PartialItem {
    /// Create a new partial item.
    fn new(index: PartialIndex) -> Self;

    /// Flush the value back to the database.
    fn flush<R: RootStatus, DB: WriteBackend>(
        &mut self,
        raw: &mut Raw<R, DB::Construct>,
        db: &mut DB,
    ) -> Result<(), Error<DB::Error>> where
        DB::Construct: CompatibleConstruct;
}

/// Partialable
pub trait Partialable {
    /// Value type of the partial item.
    type Value: PartialItem;
}

macro_rules! basic_partialables {
    ( $( $t:ty ),* ) => { $(
        impl Partialable for $t {
            type Value = PartialValue<$t>;
        }
    )* }
}

basic_partialables!(u8, u16, u32, u64, u128, U256, H256);
