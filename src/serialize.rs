//! Merkle direct serialization.

use crate::{Backend, ValueOf, Error, Value};

/// Serializable type of merkle.
pub trait Serialize<DB: Backend> {
    /// Serialize this value into a list of merkle value.
    fn serialize(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>>;
}

/// Serialize a vector at given depth.
pub fn serialize_vector<DB: Backend>(values: &[ValueOf<DB>], db: &mut DB, at_depth: Option<usize>) -> Result<ValueOf<DB>, Error<DB::Error>> {
    let total_depth = at_depth.unwrap_or({
        let mut max_len = 1;
        let mut total_depth = 0;
        while max_len < values.len() {
            max_len *= 2;
            total_depth += 1;
        }
        total_depth
    });

    let mut current = values.iter().cloned().collect::<Vec<_>>();
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
