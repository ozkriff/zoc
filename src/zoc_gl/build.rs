// See LICENSE file for copyright and license details.

#![feature(old_io, old_path)]

extern crate gl_generator;
extern crate khronos_api;

#[cfg(target_os = "windows")]
const GENERATOR: gl_generator::StructGenerator = gl_generator::StructGenerator;

#[cfg(not(target_os = "windows"))]
const GENERATOR: gl_generator::StaticStructGenerator = gl_generator::StaticStructGenerator;

fn main() {
    let dest = Path::new(std::env::var("OUT_DIR").unwrap());
    let mut file = std::old_io::File::create(&dest.join("gl_bindings.rs")).unwrap();
    gl_generator::generate_bindings(
        GENERATOR,
        gl_generator::registry::Ns::Gles2,
        gl_generator::Fallbacks::None,
        khronos_api::GL_XML,
        vec![],
        "2.0",
        "core",
        &mut file,
    ).unwrap();
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
