pub fn next_power_of_two(len: usize) -> usize {
    let mut ret = 1;
    while ret < len {
        ret *= 2;
    }
    ret
}
