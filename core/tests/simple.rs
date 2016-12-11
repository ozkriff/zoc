extern crate core;
extern crate cgmath;

use cgmath::{Vector2};
use core::{
    Core,
    Command,
    CoreEvent,
    ExactPos,
    MapPos,
    SlotId,
    PlayerId,
    UnitId,
    MoveMode,
};

fn command_end_turn(core: &mut Core) {
    core.do_command(Command::EndTurn);
}

fn command_create_ground_unit_at(
    core: &mut Core,
    (pos, slot_id): (MapPos, u8),
    typename: &str,
) {
    let type_id = core.db().unit_type_id(typename);
    core.do_command(Command::CreateUnit {
        pos: ExactPos{slot_id: SlotId::Id(slot_id), map_pos: pos},
        type_id: type_id,
    });
}

fn wait_create_unit(core: &mut Core, pos: MapPos, typename: &str) -> UnitId {
    let event = core.get_event().unwrap();
    let info = match event {
        CoreEvent::CreateUnit{unit_info} => unit_info,
        _ => panic!(),
    };
    assert_eq!(info.pos.map_pos, pos);
    assert_eq!(info.type_id, core.db().unit_type_id(typename));
    info.unit_id
}

fn command_create_air_unit_at(core: &mut Core, pos: MapPos, typename: &str) {
    let type_id = core.db().unit_type_id(typename);
    core.do_command(Command::CreateUnit {
        pos: ExactPos{slot_id: SlotId::Air, map_pos: pos},
        type_id: type_id,
    });
}


fn wait_end_turn(core: &mut Core, old_player_id: i32, new_player_id: i32) {
    assert_eq!(core.get_event(), Some(CoreEvent::EndTurn {
        old_id: PlayerId{id: old_player_id},
        new_id: PlayerId{id: new_player_id},
    }));
}

fn command_move(core: &mut Core, unit_id: UnitId, path: &[(MapPos, u8)]) {
    assert!(path.len() >= 2);
    let path = path.iter().map(|&(pos, slot_id)| {
        ExactPos{slot_id: SlotId::Id(slot_id), map_pos: pos}
    }).collect();
    core.do_command(Command::Move {
        unit_id: unit_id,
        mode: MoveMode::Fast,
        path: path,
    });
}

fn wait_move(core: &mut Core, unit_id: UnitId, path: &[(MapPos, u8)]) {
    let expected_unit_id = unit_id;
    assert!(path.len() >= 2);
    for window in path.windows(2) {
        let expected_from = ExactPos {
            map_pos: window[0].0,
            slot_id: SlotId::Id(window[0].1),
        };
        let expected_to = ExactPos {
            map_pos: window[1].0,
            slot_id: SlotId::Id(window[1].1),
        };
        match core.get_event().unwrap() {
            CoreEvent::Move{unit_id, from, to, ..} => {
                assert_eq!(unit_id, expected_unit_id);
                assert_eq!(from, expected_from);
                assert_eq!(to, expected_to);
            },
            _ => panic!(),
        }
    }
}

fn wait_show_unit(core: &mut Core, pos: MapPos, id: UnitId) {
    let event = core.get_event().unwrap();
    match event {
        CoreEvent::ShowUnit{unit_info} => {
            assert_eq!(unit_info.unit_id, id);
            assert_eq!(unit_info.pos.map_pos, pos);
        },
        _ => panic!(),
    }
}

fn command_attach(
    core: &mut Core,
    transporter_id: UnitId,
    attached_unit_id: UnitId,
) {
    core.do_command(Command::Attach {
        transporter_id: transporter_id,
        attached_unit_id: attached_unit_id,
    });
}

fn wait_attach(core: &mut Core, id1: UnitId, id2: UnitId) {
    let event = core.get_event().unwrap();
    match event {
        CoreEvent::Attach{transporter_id, attached_unit_id, ..} => {
            assert_eq!(transporter_id, id1);
            assert_eq!(attached_unit_id, id2);
        },
        _ => panic!(),
    }
}

fn basic_core() -> Core {
    Core::new(&core::Options {
        game_type: core::GameType::Hotseat,
        map_name: "map01".to_owned(),
        players_count: 2,
    })
}

#[test]
fn test_transporter_with_attached_unit_comes_out_of_fow() {
    let pos_a1 = MapPos{v: Vector2{x: 0, y: 1}};
    let pos_a2 = MapPos{v: Vector2{x: 1, y: 1}};
    let pos_a3 = MapPos{v: Vector2{x: 2, y: 1}};
    let pos_b = MapPos{v: Vector2{x: 9, y: 2}};
    let mut core = basic_core();

    assert_eq!(core.player_id(), PlayerId{id: 0});
    command_create_ground_unit_at(&mut core, (pos_a1, 0), "truck");
    let truck1_id = wait_create_unit(&mut core, pos_a1, "truck");
    command_create_ground_unit_at(&mut core, (pos_a1, 1), "truck");
    let jeep_id = wait_create_unit(&mut core, pos_a1, "truck");
    command_end_turn(&mut core);

    assert_eq!(core.player_id(), PlayerId{id: 1});
    wait_end_turn(&mut core, 0, 1);
    command_create_air_unit_at(&mut core, pos_b, "helicopter");
    let _ = wait_create_unit(&mut core, pos_b, "helicopter");
    command_end_turn(&mut core);

    assert_eq!(core.player_id(), PlayerId{id: 0});
    wait_end_turn(&mut core, 0, 1);
    wait_end_turn(&mut core, 1, 0);
    command_move(&mut core, truck1_id, &[(pos_a1, 0), (pos_a2, 0)]);
    wait_move(&mut core, truck1_id, &[(pos_a1, 0), (pos_a2, 0)]);
    command_move(&mut core, jeep_id, &[(pos_a1, 1), (pos_a2, 1), (pos_a3, 0)]);
    wait_move(&mut core, jeep_id, &[(pos_a1, 1), (pos_a2, 1), (pos_a3, 0)]);
    command_attach(&mut core, truck1_id, jeep_id);
    wait_attach(&mut core, truck1_id, jeep_id);
    command_end_turn(&mut core);

    assert_eq!(core.player_id(), PlayerId{id: 1});
    wait_end_turn(&mut core, 1, 0);
    wait_show_unit(&mut core, pos_a2, jeep_id);
    wait_move(&mut core, jeep_id, &[(pos_a2, 1), (pos_a3, 0)]);
    wait_show_unit(&mut core, pos_a2, truck1_id);
    wait_attach(&mut core, truck1_id, jeep_id);
    wait_end_turn(&mut core, 0, 1);
}
