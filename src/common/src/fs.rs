// See LICENSE file for copyright and license details.

use std::path::{Path};
use std::io::{Cursor};

#[cfg(not(target_os = "android"))]
pub fn load<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    use std::fs::{File};
    use std::io::{Read};

    let mut buf = Vec::new();
    let mut file = File::open(&Path::new("assets").join(&path))
        .ok().expect("Can`t open file");
    file.read_to_end(&mut buf).ok().expect("Can`t open file");
    Cursor::new(buf)
}

#[cfg(target_os = "android")]
pub fn load<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    use android_glue;

    let filename = path.as_ref().to_str()
        .expect("Can`t convert Path to &str");
    let buf = android_glue::load_asset(filename)
        .ok().expect("Can`t load asset");
    Cursor::new(buf)
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
