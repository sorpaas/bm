/// Value in a merkle tree.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Value<I, E> {
    /// Intermediate, which is hash of two sub-items.
    Intermediate(I),
    /// End value of the tree.
    End(E),
}

impl<I, E> Value<I, E> {
    /// Return `Some` if this value is an intermediate, otherwise return `None`.
    pub fn intermediate(self) -> Option<I> {
        match self {
            Value::Intermediate(intermediate) => Some(intermediate),
            Value::End(_) => None,
        }
    }

    /// Return `Some` if this value is an end value, otherwise return `None`.
    pub fn end(self) -> Option<E> {
        match self {
            Value::Intermediate(_) => None,
            Value::End(end) => Some(end),
        }
    }
}

impl<I: AsRef<[u8]>, E: AsRef<[u8]>> AsRef<[u8]> for Value<I, E> {
    fn as_ref(&self) -> &[u8] {
        match self {
            Value::Intermediate(ref intermediate) => intermediate.as_ref(),
            Value::End(ref end) => end.as_ref(),
        }
    }
}

/// Represents a basic merkle tree with a known root.
pub trait Tree {
    /// Root status of the tree.
    type RootStatus: RootStatus;
    /// Backend of the tree.
    type Backend: Backend;

    /// Root of the merkle tree.
    fn root(&self) -> ValueOf<Self::Backend>;
    /// Drop the merkle tree.
    fn drop(self, db: &mut Self::Backend) -> Result<(), Error<<Self::Backend as Backend>::Error>>;
    /// Convert the tree into a raw tree.
    fn into_raw(self) -> crate::Raw<Self::RootStatus, Self::Backend>;
}

/// A merkle tree that is similar to a vector.
pub trait Sequence: Tree {
    /// The length of the tree.
    fn len(&self) -> usize;
}

/// Root status of a merkle tree.
pub trait RootStatus {
    /// Whether it is a dangling root.
    fn is_dangling() -> bool;
    /// Whether it is an owned root.
    fn is_owned() -> bool { !Self::is_dangling() }
}

/// Dangling root status.
pub struct Dangling;

impl RootStatus for Dangling {
    fn is_dangling() -> bool { true }
}

/// Owned root status.
pub struct Owned;

impl RootStatus for Owned {
    fn is_dangling() -> bool { false }
}

/// Intermediate value of a database.
pub type IntermediateOf<DB> = <DB as Backend>::Intermediate;
/// End value of a database.
pub type EndOf<DB> = <DB as Backend>::End;
/// Value of a database.
pub type ValueOf<DB> = Value<IntermediateOf<DB>, EndOf<DB>>;

/// Set error.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error<DBError> {
    /// The database is corrupted.
    CorruptedDatabase,
    /// Backend database error.
    Backend(DBError),
}

impl<DBError> From<DBError> for Error<DBError> {
    fn from(err: DBError) -> Self {
        Error::Backend(err)
    }
}

/// Traits for a merkle database.
pub trait Backend {
    /// Intermediate value stored in this merkle database.
    type Intermediate: AsRef<[u8]> + Clone;
    /// End value stored in this merkle database.
    type End: AsRef<[u8]> + Clone + Default;
    /// Error type for DB access.
    type Error;

    /// Get the intermediate value of given left and right child.
    fn intermediate_of(&self, left: &ValueOf<Self>, right: &ValueOf<Self>) -> IntermediateOf<Self>;
    /// Get or create the empty value at given depth-to-bottom.
    fn empty_at(&mut self, depth_to_bottom: usize) -> Result<ValueOf<Self>, Self::Error>;
    /// Get an internal item by key.
    fn get(&self, key: &IntermediateOf<Self>) -> Result<(ValueOf<Self>, ValueOf<Self>), Self::Error>;
    /// Rootify a key.
    fn rootify(&mut self, key: &IntermediateOf<Self>) -> Result<(), Self::Error>;
    /// Unrootify a key.
    fn unrootify(&mut self, key: &IntermediateOf<Self>) -> Result<(), Self::Error>;
    /// Insert a new internal item.
    fn insert(&mut self, key: IntermediateOf<Self>, value: (ValueOf<Self>, ValueOf<Self>)) -> Result<(), Self::Error>;
}

/// Leakable value, whose default behavior of drop is to leak.
pub trait Leak {
    /// Metadata to represent this merkle struct.
    type Metadata;

    /// Initialize from a previously leaked value.
    fn from_leaked(metadata: Self::Metadata) -> Self;
    /// Metadata of the value.
    fn metadata(&self) -> Self::Metadata;
}
