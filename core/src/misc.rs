use std::cmp;
use rand::{thread_rng, Rng};

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

pub fn get_shuffled_indices<T>(v: &[T]) -> Vec<usize> {
    let mut indices: Vec<_> = (0..v.len()).collect();
    thread_rng().shuffle(&mut indices);
    indices
}

#[cfg(test)]
mod tests {
    use misc::{clamp, get_shuffled_indices};

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(-1, 0, 10), 0);
        assert_eq!(clamp(11, 0, 10), 10);
        assert_eq!(clamp(0, 0, 10), 0);
        assert_eq!(clamp(10, 0, 10), 10);
        assert_eq!(clamp(5, 0, 10), 5);
    }

    #[test]
    fn test_shuffle_touches_all_fields() {
        let mut v = [false; 10];
        let indices = get_shuffled_indices(&v);
        for i in indices {
            v[i] = true;
        }
        for n in &v {
            assert_eq!(*n, true);
        }
    }
}
