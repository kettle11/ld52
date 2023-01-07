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
