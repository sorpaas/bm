/// Value in a merkle tree.
#[derive(Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "parity-scale-codec", derive(parity_scale_codec::Encode, parity_scale_codec::Decode))]
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

/// Construct for a merkle tree.
pub trait Construct: Sized {
    /// Intermediate value stored in this merkle database.
    type Intermediate: Clone;
    /// End value stored in this merkle database.
    type End: Clone + Default;

    /// Get the intermediate value of given left and right child.
    fn intermediate_of(left: &ValueOf<Self>, right: &ValueOf<Self>) -> Self::Intermediate;
    /// Get or create the empty value given a backend. `empty_at(0)`
    /// should always equal to `Value::End(Default::default())`.
    fn empty_at<DB: WriteBackend<Construct=Self>>(
        db: &mut DB,
        depth_to_bottom: usize
    ) -> Result<ValueOf<Self>, DB::Error>;
}

/// Represents a basic merkle tree with a known root.
pub trait Tree {
    /// Root status of the tree.
    type RootStatus: RootStatus;
    /// Construct of the tree.
    type Construct: Construct;

    /// Root of the merkle tree.
    fn root(&self) -> ValueOf<Self::Construct>;
    /// Drop the merkle tree.
    fn drop<DB: WriteBackend<Construct=Self::Construct>>(
        self,
        db: &mut DB
    ) -> Result<(), Error<DB::Error>>;
    /// Convert the tree into a raw tree.
    fn into_raw(self) -> crate::Raw<Self::RootStatus, Self::Construct>;
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

/// Value of a construct.
pub type ValueOf<C> = Value<<C as Construct>::Intermediate, <C as Construct>::End>;

/// Set error.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error<DBError> {
    /// The database is corrupted.
    CorruptedDatabase,
    /// Value trying to access overflowed the list or vector.
    AccessOverflowed,
    /// Parameters are invalid.
    InvalidParameter,
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
    /// Construct of the backend.
    type Construct: Construct;
    /// Error type for DB access.
    type Error;
}

/// Read backend.
pub trait ReadBackend: Backend {
    /// Get an internal item by key.
    fn get(
        &mut self,
        key: &<Self::Construct as Construct>::Intermediate,
    ) -> Result<(ValueOf<Self::Construct>, ValueOf<Self::Construct>), Self::Error>;
}

/// Write backend.
pub trait WriteBackend: ReadBackend {
    /// Rootify a key.
    fn rootify(
        &mut self,
        key: &<Self::Construct as Construct>::Intermediate,
    ) -> Result<(), Self::Error>;
    /// Unrootify a key.
    fn unrootify(
        &mut self,
        key: &<Self::Construct as Construct>::Intermediate,
    ) -> Result<(), Self::Error>;
    /// Insert a new internal item.
    fn insert(
        &mut self,
        key: <Self::Construct as Construct>::Intermediate,
        value: (ValueOf<Self::Construct>, ValueOf<Self::Construct>)
    ) -> Result<(), Self::Error>;
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
