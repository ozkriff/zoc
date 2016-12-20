use std::cmp;
use std::sync::mpsc::{Receiver};
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

pub fn rx_collect<T>(rx: &Receiver<T>) -> Vec<T> {
    let mut v = Vec::new();
    while let Ok(data) = rx.try_recv() {
        v.push(data);
    }
    v
}

pub fn opt_rx_collect<T>(rx: &Option<Receiver<T>>) -> Vec<T> {
    if let Some(ref rx) = *rx {
        rx_collect(rx)
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{channel};
    use misc::{clamp, get_shuffled_indices, rx_collect, opt_rx_collect};

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

    #[test]
    fn test_rx_collect() {
        let (tx, rx) = channel();
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();
        assert_eq!(rx_collect(&rx), [1, 2, 3]);
    }

    #[test]
    fn test_opt_rx_collect() {
        let (tx, rx) = channel();
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();
        assert_eq!(opt_rx_collect(&Some(rx)), [1, 2, 3]);
    }
}
