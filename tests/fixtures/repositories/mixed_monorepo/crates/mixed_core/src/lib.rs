pub fn shared_fingerprint(input: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in input.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_for_same_input() {
        assert_eq!(shared_fingerprint("rune"), shared_fingerprint("rune"));
    }
}
