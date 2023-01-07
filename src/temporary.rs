use crate::*;

/// Despawned during `pre_fixed_update_systems` at the start of the next frame.
#[derive(Component, Clone)]
pub struct Temporary(pub usize);

pub fn despawn_temporaries(world: &mut World) {
    let mut to_despawn = Vec::new();

    for (e, temporary) in world.query::<&mut Temporary>().iter() {
        if temporary.0 == 0 {
            to_despawn.push(e);
        }
        temporary.0 = temporary.0.saturating_sub(1);
    }

    for e in to_despawn {
        let _ = world.despawn(e);
    }
}

pub struct DelayedAction {
    pub time: f32,
    thing_to_do: Box<dyn Fn(&mut World, &mut Resources) + Send + Sync>,
}

impl DelayedAction {
    pub fn new(f: impl Fn(&mut World, &mut Resources) + Send + Sync + 'static, time: f32) -> Self {
        Self {
            time,
            thing_to_do: Box::new(f),
        }
    }
}

pub fn run_delayed_actions(world: &mut World, resources: &mut Resources) {
    let mut to_despawn = Vec::new();
    let time_elapsed = resources.get::<Time>().fixed_time_step_seconds as f32;

    for (e, t) in world.query::<&mut DelayedAction>().iter() {
        t.time -= time_elapsed;
        if t.time <= 0.0 {
            to_despawn.push(e);
        }
    }

    for e in to_despawn {
        let f = world.remove_one::<DelayedAction>(e).unwrap();
        (f.thing_to_do)(world, resources);
        world.despawn(e);
    }
}
