use crate::{RootStatus, Backend, Sequence, Raw, Dangling, Error, ValueOf, EndOf, Index, Leak, Value, Tree, Owned};

const LEN_INDEX: Index = Index::root().right();
const ITEM_ROOT_INDEX: Index = Index::root().left();

/// A tree with length mixed in.
pub struct LengthMixed<R: RootStatus, DB: Backend, S: Sequence<Backend=DB, RootStatus=Dangling>> {
    raw: Raw<R, DB>,
    inner: S,
}

impl<R: RootStatus, DB: Backend, S> LengthMixed<R, DB, S> where
    S: Sequence<Backend=DB, RootStatus=Dangling>,
    EndOf<DB>: From<usize> + Into<usize>,
{
    /// Reconstruct the mixed-length tree.
    pub fn reconstruct<F>(root: ValueOf<DB>, db: &mut DB, f: F) -> Result<Self, Error<DB::Error>> where
        F: FnOnce(Raw<Dangling, DB>, &DB, usize) -> Result<S, Error<DB::Error>>,
    {
        let raw = Raw::<R, DB>::from_leaked(root);
        let len: usize = raw.get(db, LEN_INDEX)?
            .ok_or(Error::CorruptedDatabase)?
            .end()
            .ok_or(Error::CorruptedDatabase)?
            .into();
        let inner_raw = raw.subtree(db, ITEM_ROOT_INDEX)?;

        let inner = f(inner_raw, db, len)?;
        Ok(Self { inner, raw })
    }

    /// Deconstruct the mixed-length tree.
    pub fn deconstruct(self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        self.raw.get(db, LEN_INDEX)?;
        self.raw.get(db, ITEM_ROOT_INDEX)?;
        Ok(self.raw.root())
    }

    /// Call with the inner sequence.
    pub fn with<RT, F>(&self, db: &mut DB, f: F) -> Result<RT, Error<DB::Error>> where
        F: FnOnce(&S, &mut DB) -> Result<RT, Error<DB::Error>>
    {
        f(&self.inner, db)
    }

    /// Call with a mutable reference to the inner sequence.
    pub fn with_mut<RT, F>(&mut self, db: &mut DB, f: F) -> Result<RT, Error<DB::Error>> where
        F: FnOnce(&mut S, &mut DB) -> Result<RT, Error<DB::Error>>
    {
        let ret = f(&mut self.inner, db)?;
        let new_len = self.inner.len();
        let new_inner_root = self.inner.root();

        self.raw.set(db, ITEM_ROOT_INDEX, new_inner_root)?;
        self.raw.set(db, LEN_INDEX, Value::End(new_len.into()))?;

        Ok(ret)
    }
}

impl<DB: Backend, S> LengthMixed<Owned, DB, S> where
    S: Sequence<Backend=DB, RootStatus=Dangling> + Leak,
    EndOf<DB>: From<usize> + Into<usize>,
{
    /// Create a new mixed-length tree.
    pub fn create<OS, F>(db: &mut DB, f: F) -> Result<Self, Error<DB::Error>> where
        F: FnOnce(&mut DB) -> Result<OS, Error<DB::Error>>,
        OS: Sequence<Backend=DB> + Leak<Metadata=S::Metadata>,
    {
        let inner = f(db)?;
        let len = inner.len();
        let mut raw = Raw::default();

        raw.set(db, ITEM_ROOT_INDEX, inner.root())?;
        raw.set(db, LEN_INDEX, Value::End(len.into()))?;
        let metadata = inner.metadata();
        inner.drop(db)?;
        let dangling_inner = S::from_leaked(metadata);

        Ok(Self { raw, inner: dangling_inner })
    }
}

impl<R: RootStatus, DB: Backend, S> Tree for LengthMixed<R, DB, S> where
    S: Sequence<Backend=DB, RootStatus=Dangling>,
{
    type RootStatus = R;
    type Backend = DB;

    fn root(&self) -> ValueOf<Self::Backend> {
        self.raw.root()
    }

    fn drop(self, db: &mut DB) -> Result<(), Error<DB::Error>> {
        self.inner.drop(db)?;
        self.raw.drop(db)?;
        Ok(())
    }

    fn into_raw(self) -> Raw<R, DB> {
        self.raw
    }
}

impl<R: RootStatus, DB: Backend, S> Sequence for LengthMixed<R, DB, S> where
    S: Sequence<Backend=DB, RootStatus=Dangling>,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<R: RootStatus, DB: Backend, S> Leak for LengthMixed<R, DB, S> where
    S: Sequence<Backend=DB, RootStatus=Dangling> + Leak,
{
    type Metadata = (ValueOf<DB>, S::Metadata);

    fn metadata(&self) -> Self::Metadata {
        let inner_metadata = self.inner.metadata();
        let raw_metadata = self.raw.metadata();

        (raw_metadata, inner_metadata)
    }

    fn from_leaked((raw_metadata, inner_metadata): Self::Metadata) -> Self {
        Self {
            raw: Raw::from_leaked(raw_metadata),
            inner: S::from_leaked(inner_metadata),
        }
    }
}
