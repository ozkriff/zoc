// See LICENSE file for copyright and license details.

use std::io::{BufRead};
use std::path::{Path};
use std::str::{Words, Split, FromStr};
use cgmath::{Vector3, Vector2};
use common::types::{ZInt, ZFloat};
use common::fs;
use types::{VertexCoord, TextureCoord, Normal};

struct Face {
    vertex: [ZInt; 3],
    texture: [ZInt; 3],
    normal: [ZInt; 3],
}

pub struct Model {
    coords: Vec<VertexCoord>,
    normals: Vec<Normal>,
    texture_coords: Vec<TextureCoord>,
    faces: Vec<Face>,
}

fn parse_word<T: FromStr>(words: &mut Words) -> T {
    let str = words.next().expect("Can not read next word");
    str.parse().ok().expect("Can not convert from string")
}

fn parse_charsplit<T: FromStr>(words: &mut Split<char>) -> T {
    let str = words.next().expect("Can not read next word");
    str.parse().ok().expect("Can not convert from string")
}

impl Model {
    pub fn new(path: &Path) -> Model {
        let mut obj = Model {
            coords: Vec::new(),
            normals: Vec::new(),
            texture_coords: Vec::new(),
            faces: Vec::new(),
        };
        obj.read(path);
        obj
    }

    fn read_v(words: &mut Words) -> VertexCoord {
        VertexCoord{v: Vector3 {
            x: parse_word(words),
            // y: parse_word(words), // TODO: flip models
            y: -parse_word::<ZFloat>(words),
            z: parse_word(words),
        }}
    }

    fn read_vn(words: &mut Words) -> Normal {
        Normal{v: Vector3 {
            x: parse_word(words),
            y: parse_word(words),
            z: parse_word(words),
        }}
    }

    fn read_vt(words: &mut Words) -> TextureCoord {
        TextureCoord{v: Vector2 {
            x: parse_word(words),
            y: 1.0 - parse_word(words), // flip
        }}
    }

    fn read_f(words: &mut Words) -> Face {
        let mut face = Face {
            vertex: [0, 0, 0],
            texture: [0, 0, 0],
            normal: [0, 0, 0],
        };
        let mut i = 0;
        for group in words.by_ref() {
            let mut w = group.split('/');
            face.vertex[i] = parse_charsplit(&mut w);
            face.texture[i] = parse_charsplit(&mut w);
            face.normal[i] = parse_charsplit(&mut w);
            i += 1;
        }
        face
    }

    fn read_line(&mut self, line: &str) {
        let mut words = line.words();
        fn is_correct_tag(tag: &str) -> bool {
            tag.len() != 0 && tag.char_at(0) != '#'
        }
        match words.next() {
            Some(tag) if is_correct_tag(tag) => {
                let w = &mut words;
                match tag {
                    "v" => self.coords.push(Model::read_v(w)),
                    "vn" => self.normals.push(Model::read_vn(w)),
                    "vt" => self.texture_coords.push(Model::read_vt(w)),
                    "f" => self.faces.push(Model::read_f(w)),
                    _ => {},
                }
            }
            _ => {},
        };
    }

    fn read(&mut self, path: &Path) {
        for line in fs::load(path).lines() {
            match line {
                Ok(line) => self.read_line(&line),
                Err(msg) => panic!("Obj: read error: {}", msg),
            }
        }
    }

    pub fn build(&self) -> Vec<VertexCoord> {
        let mut mesh = Vec::new();
        for face in self.faces.iter() {
            for i in 0 .. 3 {
                let vertex_id = face.vertex[i] as usize - 1;
                mesh.push(self.coords[vertex_id].clone());
            }
        }
        mesh
    }

    pub fn build_tex_coord(&self) -> Vec<TextureCoord> {
        let mut tex_coords = Vec::new();
        for face in self.faces.iter() {
            for i in 0 .. 3 {
                let texture_coord_id = face.texture[i] as usize - 1;
                tex_coords.push(self.texture_coords[texture_coord_id].clone());
            }
        }
        tex_coords
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
