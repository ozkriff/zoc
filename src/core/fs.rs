// See LICENSE file for copyright and license details.

use std::old_io::{MemReader};

#[cfg(not(target_os = "android"))]
pub fn load(path: &Path) -> MemReader {
    use std::old_io::fs::{File};

    let buf = File::open(path).read_to_end()
        .ok().expect("Can`t open file");
    MemReader::new(buf)
}

#[cfg(target_os = "android")]
pub fn load(path: &Path) -> MemReader {
    use android_glue;

    let filename = path.as_str()
        .expect("Can`t convert Path to &str");
    let buf = android_glue::load_asset(filename)
        .ok().expect("Can`t load asset");
    MemReader::new(buf)
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
