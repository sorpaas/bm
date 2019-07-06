//! Merkle direct serialization.

use crate::{Backend, ValueOf, Error, Value};

/// Serializable type of merkle.
pub trait Serialize<DB: Backend> {
    /// Serialize this value into a list of merkle value.
    fn serialize(&self, db: &mut DB) -> Result<Vec<ValueOf<DB>>, Error<DB::Error>>;
}

/// Serialize the given type into root.
pub fn serialize_root<DB: Backend, T: Serialize<DB>>(value: &T, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
    let serialized = value.serialize(db)?;

    let mut max_len = 1;
    let mut total_depth = 0;
    while max_len < serialized.len() {
        max_len *= 2;
        total_depth += 1;
    }

    let mut current = serialized;
    let mut next = Vec::new();
    for depth in (1..(total_depth + 1)).rev() {
        let depth_to_bottom = total_depth - depth;
        while !current.is_empty() {
            let left = current.pop().unwrap_or(db.empty_at(depth_to_bottom)?);
            let right = current.pop().unwrap_or(db.empty_at(depth_to_bottom)?);
            let key = db.intermediate_of(&left, &right);

            db.insert(key.clone(), (left, right))?;
            next.push(Value::Intermediate(key));
        }
        current = next;
        next = Vec::new();
    }

    Ok(if total_depth == 0 {
        current[0].clone()
    } else {
        next[0].clone()
    })
}
