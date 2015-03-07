// See LICENSE file for copyright and license details.

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

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
