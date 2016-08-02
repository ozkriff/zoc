// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use std::collections::hash_map::{Entry};
use std::fmt::{Debug};
use std::io::{BufRead};
use std::path::{Path};
use std::str::{SplitWhitespace, Split, FromStr};
use fs;
use pipeline::{Vertex};

type Face = [[u16; 3]; 3];

struct Line {
    vertex: [u16; 2],
}

type Uv = [f32; 2];

type Pos = [f32; 3];

pub struct Model {
    faces: Vec<Face>,
    lines: Vec<Line>,
    uvs: Vec<Uv>,
    positions: Vec<Pos>,
}

fn parse_word<T: FromStr>(words: &mut SplitWhitespace) -> T
    where T::Err: Debug
{
    let str = words.next().expect("Can not read next word");
    str.parse().expect("Can not parse word")
}

fn parse_charsplit<T: FromStr>(words: &mut Split<char>) -> T
    where T::Err: Debug
{
    let str = words.next().expect("Can not read next word");
    str.parse().expect("Can not parse word")
}

impl Model {
    pub fn new<P: AsRef<Path>>(path: P) -> Model {
        let mut obj = Model {
            positions: Vec::new(),
            uvs: Vec::new(),
            faces: Vec::new(),
            lines: Vec::new(),
        };
        obj.read(path);
        obj
    }

    fn read_v(words: &mut SplitWhitespace) -> Pos {
        // TODO: flip models
        [parse_word(words), -parse_word::<f32>(words), parse_word(words)]
    }

    fn read_vt(words: &mut SplitWhitespace) -> Uv {
        [
            parse_word(words),
            1.0 - parse_word::<f32>(words), // flip
        ]
    }

    fn read_f(words: &mut SplitWhitespace) -> Face {
        let mut f = [[0; 3]; 3];
        for (i, group) in words.by_ref().enumerate() {
            let w = &mut group.split('/');
            f[i] = [
                parse_charsplit(w),
                parse_charsplit(w),
                parse_charsplit(w),
            ];
        }
        f
    }

    fn read_l(words: &mut SplitWhitespace) -> Line {
        Line {
            vertex: [
                parse_word(words),
                parse_word(words),
            ],
        }
    }

    fn read_line(&mut self, line: &str) {
        let mut words = line.split_whitespace();
        fn is_correct_tag(tag: &str) -> bool {
            !tag.is_empty() && !tag.starts_with('#')
        }
        match words.next() {
            Some(tag) if is_correct_tag(tag) => {
                let w = &mut words;
                match tag {
                    "v" => self.positions.push(Model::read_v(w)),
                    "vt" => self.uvs.push(Model::read_vt(w)),
                    "f" => self.faces.push(Model::read_f(w)),
                    "l" => self.lines.push(Model::read_l(w)),
                    "vn" |
                    "s" |
                    "#" => {},
                    unexpected_tag => {
                        println!("obj: unexpected tag: {}", unexpected_tag);
                    }
                }
            }
            _ => {},
        };
    }

    fn read<P: AsRef<Path>>(&mut self, path: P) {
        for line in fs::load(path).lines() {
            match line {
                Ok(line) => self.read_line(&line),
                Err(msg) => panic!("Obj: read error: {}", msg),
            }
        }
    }

    pub fn is_wire(&self) -> bool {
        !self.lines.is_empty()
    }
}

pub fn build(model: &Model) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut components_map: HashMap<(u16, u16), u16> = HashMap::new();
    for face in &model.faces {
        for face_vertex in face {
            let pos_id = face_vertex[0] - 1;
            let uv_id = face_vertex[1] - 1;
            let key = (pos_id, uv_id);
            let id = match components_map.entry(key) {
                Entry::Vacant(vacant) => {
                    let id = vertices.len() as u16;
                    vertices.push(Vertex {
                        pos: model.positions[pos_id as usize],
                        uv: model.uvs[uv_id as usize],
                    });
                    vacant.insert(id);
                    id
                }
                Entry::Occupied(occ) => *occ.get()
            };
            indices.push(id);
        }
    }
    for line in &model.lines {
        for line_vertex in &line.vertex {
            let pos_id = *line_vertex as usize - 1;
            vertices.push(Vertex {
                pos: model.positions[pos_id],
                uv: [0.0, 0.0],
            });
            indices.push(vertices.len() as u16 - 1);
        }
    }
    (vertices, indices)
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
