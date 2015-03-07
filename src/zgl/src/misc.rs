// See LICENSE file for copyright and license details.

pub fn add_quad_to_vec<T: Clone>(v: &mut Vec<T>, v1: T, v2: T, v3: T, v4: T) {
    v.push(v1.clone());
    v.push(v2);
    v.push(v3.clone());
    v.push(v1);
    v.push(v3);
    v.push(v4);
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
