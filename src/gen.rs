use std::path::{Path};
use cgmath::{Vector2, Array};
use core::{MapPos, Sector, MovePoints, ExactPos, Command, UnitId, PlayerId};
use core::db::{Db};
use core::pathfinder::{Pathfinder};
use core::map::{Terrain};
use core::partial_state::{PartialState};
use core::game_state::{GameState};
use core::check::{check_command};
use context::{Context};
use texture::{Texture, load_texture};
use mesh::{Mesh};
use pipeline::{Vertex};
use core::dir::{Dir, dirs};
use geom;
use fs;

pub fn get_player_color(player_id: PlayerId) -> [f32; 4] {
    match player_id.id {
        0 => [0.1, 0.1, 1.0, 1.0],
        1 => [0.0, 0.8, 0.0, 1.0],
        n => panic!("Wrong player id: {}", n),
    }
}

pub fn generate_tiles_mesh<I: IntoIterator<Item=MapPos>>(
    context: &mut Context,
    tex: Texture,
    positions: I
) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut i = 0;
    for tile_pos in positions {
        let pos = geom::map_pos_to_world_pos(tile_pos);
        for dir in dirs() {
            let vertex = geom::index_to_hex_vertex(dir.to_int());
            let uv = vertex.v.truncate() / (geom::HEX_EX_RADIUS * 2.0)
                + Vector2::from_value(0.5);
            vertices.push(Vertex {
                pos: (pos.v + vertex.v).into(),
                uv: uv.into(),
            });
        }
        indices.extend_from_slice(&[
            i, i + 1, i + 2,
            i, i + 2, i + 3,
            i, i + 3, i + 5,
            i + 3, i + 4, i + 5,
        ]);
        i += 6;
    }
    Mesh::new(context, &vertices, &indices, tex)
}

pub fn generate_sector_mesh(context: &mut Context, sector: &Sector, tex: Texture) -> Mesh {
    generate_tiles_mesh(context, tex, sector.positions.to_vec())
}

pub fn generate_map_mesh(context: &mut Context, state: &PartialState, tex: Texture) -> Mesh {
    let mut normal_positions = Vec::new();
    for tile_pos in state.map().get_iter() {
        if *state.map().tile(tile_pos) != Terrain::Water {
            normal_positions.push(tile_pos);
        }
    }
    generate_tiles_mesh(context, tex, normal_positions)
}

pub fn generate_water_mesh(context: &mut Context, state: &PartialState, tex: Texture) -> Mesh {
    let mut normal_positions = Vec::new();
    for pos in state.map().get_iter() {
        if *state.map().tile(pos) == Terrain::Water {
            normal_positions.push(pos);
        }
    }
    generate_tiles_mesh(context, tex, normal_positions)
}

pub fn empty_mesh(context: &mut Context) -> Mesh {
    Mesh::new_wireframe(context, &[], &[])
}

pub fn build_walkable_mesh(
    context: &mut Context,
    pf: &Pathfinder,
    state: &PartialState,
    move_points: MovePoints,
) -> Mesh {
    let map = state.map();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut i = 0;
    for tile_pos in map.get_iter() {
        if pf.get_map().tile(tile_pos).cost().n > move_points.n {
            continue;
        }
        if let Some(parent_dir) = pf.get_map().tile(tile_pos).parent() {
            let tile_pos_to = Dir::get_neighbour_pos(tile_pos, parent_dir);
            let exact_pos = ExactPos {
                map_pos: tile_pos,
                slot_id: pf.get_map().tile(tile_pos).slot_id(),
            };
            let exact_pos_to = ExactPos {
                map_pos: tile_pos_to,
                slot_id: pf.get_map().tile(tile_pos_to).slot_id(),
            };
            let mut world_pos_from = geom::exact_pos_to_world_pos(state, exact_pos);
            world_pos_from.v.z = 0.0;
            let mut world_pos_to = geom::exact_pos_to_world_pos(state, exact_pos_to);
            world_pos_to.v.z = 0.0;
            vertices.push(Vertex {
                pos: geom::lift(world_pos_from.v).into(),
                uv: [0.5, 0.5],
            });
            vertices.push(Vertex {
                pos: geom::lift(world_pos_to.v).into(),
                uv: [0.5, 0.5],
            });
            indices.extend_from_slice(&[i, i + 1]);
            i += 2;
        }
    }
    Mesh::new_wireframe(context, &vertices, &indices)
}

pub fn build_targets_mesh(db: &Db, context: &mut Context, state: &PartialState, unit_id: UnitId) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let unit = state.unit(unit_id);
    let mut i = 0;
    for (&enemy_id, enemy) in state.units() {
        if unit.player_id == enemy.player_id {
            continue;
        }
        let command = Command::AttackUnit {
            attacker_id: unit_id,
            defender_id: enemy_id,
        };
        if !check_command(db, unit.player_id, state, &command).is_ok() {
            continue;
        }
        let world_pos_from = geom::exact_pos_to_world_pos(state, unit.pos);
        let world_pos_to = geom::exact_pos_to_world_pos(state, enemy.pos);
        vertices.push(Vertex {
            pos: geom::lift(world_pos_from.v).into(),
            uv: [0.5, 0.5],
        });
        vertices.push(Vertex {
            pos: geom::lift(world_pos_to.v).into(),
            uv: [0.5, 0.5],
        });
        indices.extend_from_slice(&[i, i + 1]);
        i += 2;
    }
    Mesh::new_wireframe(context, &vertices, &indices)
}

pub fn get_shell_mesh(context: &mut Context) -> Mesh {
    let w = 0.05;
    let l = w * 3.0;
    let h = 0.1;
    let vertices = [
        Vertex{pos: [-w, -l, h], uv: [0.0, 0.0]},
        Vertex{pos: [-w, l, h], uv: [0.0, 1.0]},
        Vertex{pos: [w, l, h], uv: [1.0, 0.0]},
        Vertex{pos: [w, -l, h], uv: [1.0, 0.0]},
    ];
    let indices = [0, 1, 2, 2, 3, 0];
    let texture_data = fs::load("shell.png").into_inner();
    let texture = load_texture(context, &texture_data);
    Mesh::new(context, &vertices, &indices, texture)
}

pub fn get_road_mesh(context: &mut Context) -> Mesh {
    let w = geom::HEX_EX_RADIUS * 0.3;
    let l = geom::HEX_EX_RADIUS;
    let h = geom::MIN_LIFT_HEIGHT / 2.0;
    let vertices = [
        Vertex{pos: [-w, -l, h], uv: [0.0, 0.0]},
        Vertex{pos: [-w, l, h], uv: [0.0, 1.0]},
        Vertex{pos: [w, l, h], uv: [1.0, 1.0]},
        Vertex{pos: [w, -l, h], uv: [1.0, 0.0]},
    ];
    let indices = [0, 1, 2, 2, 3, 0];
    let texture_data = fs::load("road.png").into_inner();
    let texture = load_texture(context, &texture_data);
    Mesh::new(context, &vertices, &indices, texture)
}

pub fn get_marker<P: AsRef<Path>>(context: &mut Context, tex_path: P) -> Mesh {
    let n = 0.2;
    let vertices = [
        Vertex{pos: [-n, 0.0, 0.1], uv: [0.0, 0.0]},
        Vertex{pos: [0.0, n * 1.4, 0.1], uv: [1.0, 0.0]},
        Vertex{pos: [n, 0.0, 0.1], uv: [0.5, 0.5]},
    ];
    let indices = [0, 1, 2];
    let texture_data = fs::load(tex_path).into_inner();
    let texture = load_texture(context, &texture_data);
    Mesh::new(context, &vertices, &indices, texture)
}

pub fn get_one_tile_mesh(context: &mut Context, texture: Texture) -> Mesh {
    let mut vertices = Vec::new();
    for dir in dirs() {
        let vertex = geom::index_to_hex_vertex(dir.to_int());
        let uv = vertex.v.truncate() / (geom::HEX_EX_RADIUS * 2.0)
            + Vector2::from_value(0.5);
        vertices.push(Vertex {
            pos: vertex.v.into(),
            uv: uv.into(),
        });
    }
    let indices = [
        0, 1, 2,
        0, 2, 3,
        0, 3, 5,
        3, 4, 5,
    ];
    Mesh::new(context, &vertices, &indices, texture)
}
