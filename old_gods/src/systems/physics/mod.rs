/// Manages:
/// * maintaining the cardinal direction an object is/was last moving in
use specs::prelude::*;

use super::super::prelude::{AABBTree, Cardinal, Exile, FPSCounter, Shape, ZLevel, AABB, V2};


// TODO: Mass and acceleration for physical bodies.

#[derive(Debug, Clone, PartialEq)]
pub struct Position(pub V2);


impl Component for Position {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}


#[derive(Debug, Clone, PartialEq)]
pub struct Velocity(pub V2);


impl Component for Velocity {
    type Storage = HashMapStorage<Self>;
}


#[derive(Debug, Clone)]
pub struct Barrier;


impl Barrier {
    pub fn tiled_type() -> String {
        "barrier".to_string()
    }
}


impl Component for Barrier {
    type Storage = HashMapStorage<Self>;
}


pub struct Physics {
    pub shape_reader: Option<ReaderId<ComponentEvent>>,
    pub position_reader: Option<ReaderId<ComponentEvent>>,
}


impl Default for Physics {
    fn default() -> Physics {
        Physics {
            shape_reader: None,
            position_reader: None,
        }
    }
}


#[derive(SystemData)]
pub struct PhysicsSystemData<'a> {
    aabb_tree: Write<'a, AABBTree>,
    barriers: ReadStorage<'a, Barrier>,
    cardinals: WriteStorage<'a, Cardinal>,
    entities: Entities<'a>,
    exiles: ReadStorage<'a, Exile>,
    fps: Read<'a, FPSCounter>,
    positions: WriteStorage<'a, Position>,
    shapes: ReadStorage<'a, Shape>,
    velocities: ReadStorage<'a, Velocity>,
    zlevels: ReadStorage<'a, ZLevel>,
}


impl<'a> PhysicsSystemData<'a> {
    /// Move all the things that can move.
    pub fn move_things(&mut self) {
        let dt = self.fps.last_delta();
        for (ent, vel, ()) in (&self.entities, &self.velocities, !&self.exiles).join() {
            let v = vel.0;
            let dxy = v.scalar_mul(dt);
            if dxy.magnitude() > 0.0 {
                let pos = self
                    .positions
                    .get_mut(ent)
                    .expect("Entity must have a position to add velocity to.");
                pos.0 += dxy;
                // Update the direction the thing is moving in
                if let Some(c) = Cardinal::from_v2(&v) {
                    self.cardinals
                        .insert(ent, c)
                        .expect("Could not insert a Cardinal dir");
                }
            }
        }
    }

    /// For each entity that has a position, barrier, shape, zlevel and velocity -
    /// find any collisions and deal with them.
    /// Only adjust the positions of entities that have a velocity, that way tiles
    /// with overlapping borders will not be moved around.
    pub fn collide_things(&mut self) {
        for (ent, _, _, shape, &ZLevel(z), ()) in (
            &self.entities,
            &self.velocities,
            &self.barriers,
            &self.shapes,
            &self.zlevels,
            !&self.exiles,
        )
            .join()
        {
            let may_pos = self.positions.get(ent);
            if may_pos.is_none() {
                continue;
            }
            let pos = may_pos.expect("Impossible").0;
            // Query all collisions with this entity's shape.
            // Find the new position using the minimum translation vector that pushes
            // it out of intersection.
            //
            // If the resulting position is different from the previous, update the
            // position.
            let new_position = self
                .aabb_tree
                .query_intersecting_barriers(
                    &self.entities,
                    &ent,
                    &self.shapes,
                    &self.positions,
                    &self.barriers,
                )
                .into_iter()
                .fold(pos, |new_pos, (other_ent, _, mtv)| {
                    let other_z = self.zlevels.get(other_ent);
                    let should_include =
                        // The other thing must have a zlevel
                        other_z.is_some()
                        // The two things must be on the same zlevel.
                        && (z - other_z.unwrap().0).abs() < f32::EPSILON
                        // The other thing must not be exiled.
                        && !self.exiles.contains(other_ent);
                    if !should_include {
                        return new_pos;
                    }

                    new_pos - mtv
                });

            if pos != new_position {
                let pos = self.positions.get_mut(ent).expect("Impossible");
                pos.0 = new_position;
                // TODO: Check if this is necessary
                self.aabb_tree
                    .insert(ent, shape.aabb().translate(&new_position));
            }
        }
    }
}


impl<'a> System<'a> for Physics {
    type SystemData = PhysicsSystemData<'a>;

    fn setup(&mut self, world: &mut World) {
        Self::SystemData::setup(world);
        // Get the Barrier and Position storages and start watching them for changes.
        let mut shapes: WriteStorage<Shape> = SystemData::fetch(&world);
        self.shape_reader = Some(shapes.register_reader());
        let mut positions: WriteStorage<Position> = SystemData::fetch(&world);
        self.position_reader = Some(positions.register_reader());
    }

    fn run(&mut self, mut data: PhysicsSystemData) {
        data.move_things();
        data.collide_things();

        // Maintain our aabb_tree with new positions and shapes
        let shape_reader = self
            .shape_reader
            .as_mut()
            .expect("Could not unwrap shape reader");
        let position_reader = self
            .position_reader
            .as_mut()
            .expect("Could not unwrap position reader");
        let positions = &data.positions;
        let shapes = &data.shapes;
        let mut events: Vec<&ComponentEvent> = shapes.channel().read(shape_reader).collect();
        events.extend(
            positions
                .channel()
                .read(position_reader)
                .collect::<Vec<_>>(),
        );
        data.aabb_tree.update_tree(
            &data.entities,
            events,
            |ent: Entity| -> Option<(Entity, AABB)> {
                let position = positions.get(ent).map(|p| p.0)?;
                shapes
                    .get(ent)
                    .map(|shape| (ent, shape.aabb().translate(&position)))
            },
        );
    }
}
