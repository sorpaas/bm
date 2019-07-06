//! Merkle direct serialization.

use crate::{Backend, ValueOf, Error};

/// Serializable type of merkle.
pub trait Serialize<DB: Backend> {
    /// Serialize this value into a list of merkle value.
    fn serialize(&self, db: &mut DB) -> Result<Vec<ValueOf<DB>>, Error<DB::Error>>;
}
