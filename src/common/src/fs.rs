// See LICENSE file for copyright and license details.

use std::path::{Path};
use std::io::{Cursor};

#[cfg(not(target_os = "android"))]
pub fn load<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    use std::fs::{File};
    use std::io::{Read};

    let mut buf = Vec::new();
    let fullpath = &Path::new("assets").join(&path);
    let mut file = match File::open(&fullpath) {
        Ok(file) => file,
        Err(err) => {
            panic!("Can`t open file '{}' ({})", fullpath.display(), err);
        },
    };
    match file.read_to_end(&mut buf) {
        Ok(_) => Cursor::new(buf),
        Err(err) => {
            panic!("Can`t read file '{}' ({})", fullpath.display(), err);
        },
    }
}

#[cfg(target_os = "android")]
pub fn load<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    use android_glue;

    let filename = path.as_ref().to_str()
        .expect("Can`t convert Path to &str");
    match android_glue::load_asset(filename) {
        Ok(buf) => Cursor::new(buf),
        // TODO: more info about error
        Err(_) => panic!("Can`t load asset '{}'", filename),
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
