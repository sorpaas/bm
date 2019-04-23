use generic_array::ArrayLength;
use core::ops::Range;
use core::cmp;

pub fn coverings<Host: ArrayLength<u8>, Value: ArrayLength<u8>>(value_index: usize) -> (usize, Vec<Range<usize>>) {
    let host_len = Host::to_usize();
    let value_len = Value::to_usize();

    let bytes = value_len * value_index;
    let host_index = bytes / host_len;
    let offset = bytes - host_len * host_index;

    let mut ranges = Vec::new();
    ranges.push(offset..cmp::min(offset + value_len, host_len));
    let mut covered = cmp::min(offset + value_len, host_len) - offset;

    while covered < value_len {
        let rest = value_len - covered;
        ranges.push(0..cmp::min(rest, host_len));
        covered += cmp::min(rest, host_len);
    }

    (host_index, ranges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use typenum::{U8, U32};

    #[test]
    fn test_coverings() {
        assert_eq!(coverings::<U32, U8>(3), (0, vec![24..32]));
        assert_eq!(coverings::<U32, U8>(4), (1, vec![0..8]));
        assert_eq!(coverings::<U8, U32>(1), (4, vec![0..8, 0..8, 0..8, 0..8]));
    }
}
