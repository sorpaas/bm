//! Utilities

use crate::{Construct, WriteBackend, Error};
use alloc::collections::VecDeque;
use generic_array::ArrayLength;

/// Required depth of given length.
pub fn required_depth(len: u64) -> usize {
    let mut max_len = 1;
    let mut total_depth = 0;
    while max_len < len {
        max_len *= 2;
        total_depth += 1;
    }
    total_depth
}

/// Serialize a vector at given depth.
pub fn vector_tree<DB: WriteBackend>(values: &[<DB::Construct as Construct>::Value], db: &mut DB, max_len: Option<u64>) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> {
    let total_depth = required_depth(max_len.unwrap_or(values.len() as u64));

    let mut current = values.iter().cloned().collect::<VecDeque<_>>();
    let mut next = VecDeque::new();
    for depth in (1..(total_depth + 1)).rev() {
        let depth_to_bottom = total_depth - depth;
        while !current.is_empty() {
            let left = current.pop_front().unwrap_or(<DB::Construct as Construct>::empty_at(db, depth_to_bottom)?);
            let right = current.pop_front().unwrap_or(<DB::Construct as Construct>::empty_at(db, depth_to_bottom)?);

            let key = <DB::Construct as Construct>::intermediate_of(&left, &right);

            db.insert(key.clone(), (left, right))?;
            next.push_back(key);
        }
        current = next;
        next = VecDeque::new();
    }

    if current.is_empty() {
        Ok(<DB::Construct as Construct>::empty_at(db, total_depth)?)
    } else {
        Ok(current[0].clone())
    }
}

/// Get the host len of a packed vector.
pub fn host_max_len<Host: ArrayLength<u8>, Value: ArrayLength<u8>>(value_len: u64) -> u64 {
    let host_array_len = Host::to_u64();
    let value_array_len = Value::to_u64();

    let bytes = value_array_len * value_len;
    if bytes % host_array_len == 0 {
        bytes / host_array_len
    } else {
        bytes / host_array_len + 1
    }
}

/// Get the host len of a packed vector.
pub fn host_len<Host: ArrayLength<u8>, Value: ArrayLength<u8>>(value_len: usize) -> usize {
    host_max_len::<Host, Value>(value_len as u64) as usize
}
