/// Manages:
/// * moving players based on their controllers' axes
use log::{trace, warn};
use specs::prelude::*;

use super::super::prelude::{Exile, MaxSpeed, Object, Player, PlayerControllers, Velocity, V2};


/// Players the movement and actions taken by characters.
pub struct PlayerSystem;


#[derive(SystemData)]
pub struct PlayerSystemData<'a> {
    entities: Entities<'a>,
    player_controllers: Read<'a, PlayerControllers>,
    players: WriteStorage<'a, Player>,
    exiles: ReadStorage<'a, Exile>,
    max_speeds: ReadStorage<'a, MaxSpeed>,
    objects: WriteStorage<'a, Object>,
    velocities: WriteStorage<'a, Velocity>,
}


/// The PlayerSystem carries out motivations on behalf of toons.
impl<'a> System<'a> for PlayerSystem {
    type SystemData = PlayerSystemData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        // Find any objects with character types so we can create player components.
        let mut deletes = vec![];
        for (ent, obj) in (&data.entities, &data.objects).join() {
            if let "character" = obj.type_is.as_ref() {
                let properties = obj.json_properties();
                trace!("character {:#?}", obj);
                let scheme = properties
                    .get("control")
                    .map(|v| v.as_str().map(|s| s.to_string()))
                    .flatten();
                match scheme.as_deref() {
                    Some("player") => {
                        let ndx = properties
                            .get("player_index")
                            .expect(
                                "Object must have a 'player_index' custom property for control.",
                            )
                            .as_u64()
                            .map(|u| u as usize)
                            .expect("'player_index value must be an integer");
                        let _ = data.players.insert(ent, Player(ndx as u32));
                    }

                    Some("npc") => {
                        panic!("TODO: NPC support");
                    }

                    None => {
                        panic!("character object must have a 'control' property");
                    }

                    Some(scheme) => {
                        warn!("unsupported character control scheme '{}'", scheme);
                    }
                }

                let _ = data.velocities.insert(ent, Velocity(V2::origin()));
                deletes.push(ent);
            }
        }
        deletes.into_iter().for_each(|ent| {
            let _ = data.objects.remove(ent);
        });

        // Run over all players and enforce their motivations.
        let joints: Vec<_> = (&data.entities, &data.players, !&data.exiles)
            .join()
            .map(|(ep, p, ())| (ep, p.clone()))
            .collect();
        for (ent, player) in joints.into_iter() {
            let v = data
                .velocities
                .get_mut(ent)
                .unwrap_or_else(|| panic!("Player {:?} does not have velocity.", player));

            let max_speed: MaxSpeed = data.max_speeds.get(ent).cloned().unwrap_or(MaxSpeed(100.0));

            // Get the player's controller on the map
            data.player_controllers.with_map_ctrl_at(player.0, |ctrl| {
                // Update the velocity of the toon based on the
                // player's controller
                let ana = ctrl.analog_rate();
                let rate = ana.unitize().unwrap_or_else(|| V2::new(0.0, 0.0));
                let mult = rate.scalar_mul(max_speed.0);
                v.0 = mult;
            });
        }
    }
}
