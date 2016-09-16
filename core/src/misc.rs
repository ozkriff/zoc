use std::cmp;

pub fn clamp<T>(n: T, min: T, max: T) -> T
    where T: Copy + cmp::PartialOrd
{
    assert!(min <= max);
    match n {
        n if n < min => min,
        n if n > max => max,
        n => n,
    }
}

#[cfg(test)]
mod tests {
    use misc::{clamp};

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(-1, 0, 10), 0);
        assert_eq!(clamp(11, 0, 10), 10);
        assert_eq!(clamp(0, 0, 10), 0);
        assert_eq!(clamp(10, 0, 10), 10);
        assert_eq!(clamp(5, 0, 10), 5);
    }
}
