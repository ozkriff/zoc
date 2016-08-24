use std::cmp;

pub fn clamp<T>(n: T, min: T, max: T) -> T
    where T: Copy + cmp::PartialOrd
{
    match n {
        n if n < min => min,
        n if n > max => max,
        n => n,
    }
}
