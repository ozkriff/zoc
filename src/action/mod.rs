use std::fmt::{Debug};
use types::{Time};
use context::{Context};
use scene::{Scene};
use unit_type_visual_info::{UnitTypeVisualInfoManager};
use mesh_manager::{MeshIdManager, MeshManager};
use camera::{Camera};

mod remove_child;
mod add_object;
mod remove_object;
mod create_unit;
mod remove_unit;
mod remove_mesh;
mod sleep;
mod rotate_to;
mod set_color;
mod change_color;
mod move_to;
mod try_fix_attached_unit;
mod detach;
mod create_text_mesh;
mod create_node;
mod remove_node;
mod sequence;
mod fork;

pub use self::sequence::Sequence;
pub use self::remove_child::RemoveChild;
pub use self::add_object::AddObject;
pub use self::remove_object::RemoveObject;
pub use self::create_unit::CreateUnit;
pub use self::remove_unit::RemoveUnit;
pub use self::remove_mesh::RemoveMesh;
pub use self::sleep::Sleep;
pub use self::rotate_to::RotateTo;
pub use self::set_color::SetColor;
pub use self::change_color::ChangeColor;
pub use self::move_to::MoveTo;
pub use self::try_fix_attached_unit::TryFixAttachedUnit;
pub use self::detach::Detach;
pub use self::create_text_mesh::CreateTextMesh;
pub use self::create_node::CreateNode;
pub use self::remove_node::RemoveNode;
pub use self::fork::Fork;

// TODO: Move to some other place
pub const WRECKS_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 1.0];

// TODO: RENAME
// TODO: Move to tactical_screen.rs?
//
// I don't like that this struct is constructed in tactical_screen.rs
// manually from fields. But I can't just pack it into some
// method because it'll create mutability problems with
// thereference to action itself.
//
// The simplest solution is to extract this fields into
// some other struct ad make it a new field of TacticalScreen.
// ........
//
// TODO: Add easing adopters-wrappers
//
pub struct ActionContext<'a> {
    // TODO: Player-specific fields
    pub camera: &'a Camera,
    pub scene: &'a mut Scene,
    // TODO: pub state: &State, // TODO: Do I need this?

    // TODO: Common fields
    pub mesh_ids: &'a MeshIdManager,
    pub context: &'a mut Context,
    pub meshes: &'a mut MeshManager,
    pub visual_info: &'a UnitTypeVisualInfoManager,
}

// TODO: action::Sequence и action::Fork
pub trait Action: Debug {
    fn is_finished(&self) -> bool { true }

    // TODO: I'm not sure that `begin\end` must mutate the scene
    // TODO: Can I get rid of begin and end somehow? Should I?
    fn begin(&mut self, _: &mut ActionContext) {}
    fn update(&mut self, _: &mut ActionContext, _: Time) {}
    fn end(&mut self, _: &mut ActionContext) {}

    // TODO: I don't like that fork needs ActionCOntext now!
    fn fork(&mut self, _: &mut ActionContext) -> Option<Box<Action>> {
        None
    }
}
