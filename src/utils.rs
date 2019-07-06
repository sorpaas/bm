//! Utilities

use crate::{Backend, ValueOf, Error, Value};

/// Required depth of given length.
pub fn required_depth(len: usize) -> usize {
    let mut max_len = 1;
    let mut total_depth = 0;
    while max_len < len {
        max_len *= 2;
        total_depth += 1;
    }
    total_depth
}

/// Serialize a vector at given depth.
pub fn vector_tree<DB: Backend>(values: &[ValueOf<DB>], db: &mut DB, at_depth: Option<usize>) -> Result<ValueOf<DB>, Error<DB::Error>> {
    let total_depth = at_depth.unwrap_or(required_depth(values.len()));

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

    Ok(current[0].clone())
}