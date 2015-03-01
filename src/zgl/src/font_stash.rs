// See LICENSE file for copyright and license details.

use std::iter::{repeat};
use std::cmp;
use std::collections::HashMap;
use stb_tt::{Font};
use cgmath::{Vector3, Vector2};
use common::types::{Size2, ZInt, ZFloat};
use common::misc::add_quad_to_vec;
use common::fs;
use texture::{Texture};
use types::{VertexCoord, TextureCoord, ScreenPos};
use mesh::{Mesh};
use zgl::{Zgl};

#[derive(Clone)]
pub struct Glyph {
    pos: ScreenPos,
    size: Size2<ZInt>,
    xoff: ZInt,
    yoff: ZInt,
}

pub struct FontStash {
    size: ZFloat,
    font: Font,
    texture: Texture,
    texture_size: ZInt,
    pos: ScreenPos,
    glyphs: HashMap<char, Glyph>,
    max_h: ZInt,
}

impl FontStash {
    pub fn new(zgl: &Zgl, font_path: &Path, size: ZFloat) -> FontStash {
        let texture_size = 1024;
        let font = Font::from_reader(&mut fs::load(font_path), size);
        let texture = Texture::new_empty(
            zgl, Size2{w: texture_size, h: texture_size});
        FontStash {
            size: size,
            font: font,
            texture: texture,
            texture_size: texture_size,
            pos: ScreenPos{v: Vector2{x: 0, y: 0}},
            glyphs: HashMap::new(),
            max_h: 0,
        }
    }

    pub fn get_glyph(&mut self, zgl: &Zgl, c: char) -> Glyph {
        if let Some(r) = self.glyphs.get(&c) {
            return r.clone();
        }
        self.add_glyph(zgl, c)
    }

    pub fn get_size(&self) -> ZFloat {
        self.size
    }

    pub fn get_text_size(&mut self, zgl: &Zgl, text: &str) -> (ScreenPos, Size2<ZInt>) {
        let mut size = Size2{w: 0, h: 0};
        let mut pos = ScreenPos{v: Vector2{x: 0, y: 0}};
        for c in text.chars() {
            let glyph = self.get_glyph(zgl, c);
            let w = glyph.size.w;
            let h = glyph.size.h;
            let yoff = -glyph.yoff;
            if pos.v.y > yoff - h {
                pos.v.y = yoff - h;
            }
            if size.h < yoff {
                size.h = yoff;
            }
            size.w += w + glyph.xoff;
        }
        (pos, size)
    }

    // TODO: fix centred hack
    pub fn get_mesh(&mut self, zgl: &Zgl, text: &str, centred: bool) -> Mesh {
        let mut vertex_data = Vec::new();
        let mut tex_data = Vec::new();
        let s = self.texture_size as ZFloat;
        let mut i = if centred {
            let (_, Size2{w, h: _}) = self.get_text_size(zgl, text);
            (-w / 2) as ZFloat
        } else {
            0.0
        };
        for c in text.chars() {
            let glyph = self.get_glyph(zgl, c);
            let w = glyph.size.w as ZFloat;
            let h = glyph.size.h as ZFloat;
            let x1 = glyph.pos.v.x as ZFloat / s;
            let y1 = glyph.pos.v.y as ZFloat / s;
            let x2 = x1 + w / s;
            let y2 = y1 + h / s;
            add_quad_to_vec(
                &mut tex_data,
                TextureCoord{v: Vector2{x: x1, y: y1}},
                TextureCoord{v: Vector2{x: x1, y: y2}},
                TextureCoord{v: Vector2{x: x2, y: y2}},
                TextureCoord{v: Vector2{x: x2, y: y1}},
            );
            let yoff = -glyph.yoff as ZFloat;
            add_quad_to_vec(
                &mut vertex_data,
                VertexCoord{v: Vector3{x: i, y: yoff, z: 0.0}},
                VertexCoord{v: Vector3{x: i, y: yoff - h, z: 0.0}},
                VertexCoord{v: Vector3{x: w + i, y: yoff - h, z: 0.0}},
                VertexCoord{v: Vector3{x: w + i, y: yoff, z: 0.0}},
            );
            i += w + glyph.xoff as ZFloat;
        }
        let mut mesh = Mesh::new(zgl, vertex_data.as_slice());
        // TODO: remove 'clone()'?
        mesh.add_texture(zgl, self.texture.clone(), tex_data.as_slice());
        mesh
    }

    fn insert_image_to_cache(
        &mut self,
        zgl: &Zgl,
        pos: ScreenPos,
        size: Size2<ZInt>,
        bitmap: Vec<u8>
    ) {
        let mut data: Vec<_> = repeat(0u8)
            .take((size.w * size.h) as usize * 4).collect();
        for y in range(0, size.h) {
            for x in range(0, size.w) {
                let n = (x + y * size.w) as usize * 4;
                data[n + 0] = 255;
                data[n + 1] = 255;
                data[n + 2] = 255;
                data[n + 3] = bitmap[(x + y * size.w) as usize];
            }
        }
        self.texture.bind(zgl);
        self.texture.set_sub_image(zgl, pos.v, size, &data);
    }

    fn start_new_row(&mut self) {
        self.pos.v.y += self.max_h;
        self.pos.v.x = 0;
        self.max_h = 0;
        assert!(self.pos.v.y < self.texture_size);
    }

    fn add_glyph(&mut self, zgl: &Zgl, c: char) -> Glyph {
        assert!(self.glyphs.get(&c).is_none());
        let index = self.font.find_glyph_index(c);
        let (bitmap, w, h, xoff, yoff) = self.font.get_glyph(index);
        if self.pos.v.x + w > self.texture_size {
            self.start_new_row();
        }
        self.pos.v.y = cmp::max(h, self.pos.v.y);
        let pos = self.pos.clone();
        let size = Size2{w: w, h: h};
        if w * h != 0 {
            self.insert_image_to_cache(zgl, pos.clone(), size.clone(), bitmap);
        }
        let xoff = if c == ' ' {
            let space_width = (self.size / 3.0) as ZInt; // TODO: get from ttf
            xoff + space_width
        } else {
            xoff
        };
        self.pos.v.x += w;
        let glyph = Glyph {
            pos: pos,
            size: size,
            xoff: xoff,
            yoff: yoff,
        };
        if self.max_h < h - yoff {
            self.max_h = h - yoff;
        }
        self.glyphs.insert(c, glyph.clone());
        glyph
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
