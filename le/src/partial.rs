use bm::{Index, Error, ReadBackend, Construct, DanglingRaw, Leak};
use primitive_types::U256;
use crate::{FromTree, CompatibleConstruct};

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
    pub fn resolve<DB: ReadBackend>(
        &self,
        root: &<DB::Construct as Construct>::Value,
        db: &mut DB
    ) -> Result<Index, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let parent = match &self.parent {
            Some(p) => p.resolve(root, db)?,
            None => Index::root(),
        };

        let (index, len) = match self.sub {
            PartialSubIndex::Raw(raw) => return Ok(parent.sub(raw)),
            PartialSubIndex::Vector(index, len) => (index, len),
            PartialSubIndex::List(index) => {
                let len = Self::get_raw::<U256, _>(root, db, parent.right())?;

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

    /// Get value of type from this partial index.
    pub fn get<T: FromTree, DB: ReadBackend>(
        &self,
        root: &<DB::Construct as Construct>::Value,
        db: &mut DB,
    ) -> Result<T, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let index = self.resolve(root, db)?;
        Self::get_raw(root, db, index)
    }

    fn get_raw<T: FromTree, DB: ReadBackend>(
        root: &<DB::Construct as Construct>::Value,
        db: &mut DB,
        index: Index,
    ) -> Result<T, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let index_root = DanglingRaw::from_leaked(root.clone()).get(db, index)?
            .ok_or(Error::CorruptedDatabase)?;

        T::from_tree(&index_root, db)
    }
}
