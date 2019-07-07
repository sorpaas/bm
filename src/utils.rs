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
pub fn vector_tree<DB: Backend>(values: &[ValueOf<DB>], db: &mut DB, max_len: Option<usize>) -> Result<ValueOf<DB>, Error<DB::Error>> {
    let total_depth = required_depth(max_len.unwrap_or(values.len()));

    let mut current = values.iter().cloned().collect::<Vec<_>>();
    let mut next = Vec::new();
    for depth in (1..(total_depth + 1)).rev() {
        let depth_to_bottom = total_depth - depth;
        while !current.is_empty() {
            let (left, right) = match current.len() {
                0 => (db.empty_at(depth_to_bottom)?, db.empty_at(depth_to_bottom)?),
                1 => {
                    let left = current.pop().expect("Length is checked to be one;
                                                     one value left; qed");
                    let right = db.empty_at(depth_to_bottom)?;
                    (left, right)
                },
                _ => {
                    let right = current.pop().expect("Length is checked to be greater than one;
                                                      At least two values left; qed");
                    let left = current.pop().expect("Length is checked to be greater than one;
                                                      At least two values left; qed");
                    (left, right)
                },
            };

            let key = db.intermediate_of(&left, &right);

            db.insert(key.clone(), (left, right))?;
            next.push(Value::Intermediate(key));
        }
        current = next;
        next = Vec::new();
    }

    Ok(current[0].clone())
}
