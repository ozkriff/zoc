// See LICENSE file for copyright and license details.

extern crate gl_generator;
extern crate khronos_api;

use std::path::{Path};

#[cfg(not(target_os = "android"))]
const GENERATOR: gl_generator::StructGenerator = gl_generator::StructGenerator;

#[cfg(target_os = "android")]
const GENERATOR: gl_generator::StaticStructGenerator = gl_generator::StaticStructGenerator;

fn main() {
    let out_dir = std::env::var("OUT_DIR")
        .expect("Can`t read OUT_DIR env var");
    let dest = Path::new(&out_dir);
    let mut file = match std::fs::File::create(&dest.join("gl_bindings.rs")) {
        Ok(file) => file,
        Err(err) => panic!("Can`t create 'gl_bindings.rs' file: {}", err),
    };
    let generate_bindings_status = gl_generator::generate_bindings(
        GENERATOR,
        gl_generator::registry::Ns::Gles2,
        gl_generator::Fallbacks::None,
        khronos_api::GL_XML,
        vec![],
        "2.0",
        "core",
        &mut file,
    );
    if let Err(err) = generate_bindings_status {
        panic!("Can`t generate gl bindings: {}", err);
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
