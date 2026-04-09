use bevy::prelude::*;
use saddle_bevy_e2e::action::Action;

pub fn set_target_pose(name: &'static str, translation: Vec3) -> Action {
    Action::Custom(Box::new(move |world| {
        let mut query = world.query::<(Entity, &Name)>();
        let entity = query
            .iter(world)
            .find(|(_, entity_name)| entity_name.as_str() == name)
            .map(|(entity, _)| entity)
            .expect("named IK target should exist");

        let mut entity_ref = world.entity_mut(entity);
        entity_ref
            .get_mut::<Transform>()
            .expect("IK target should have a Transform")
            .translation = translation;

        if let Some(mut orbit) = entity_ref.get_mut::<crate::support::OrbitMotion>() {
            orbit.center = translation;
            orbit.radius = Vec3::ZERO;
        }
    }))
}
