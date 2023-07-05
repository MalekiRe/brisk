use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;
use bevy_ecs::prelude::{Entity, NonSend, Query, Res, With, Without};
use bevy_ecs::system::Commands;
use bevy_transform::prelude::{GlobalTransform, Transform};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use stereokit::{bounds_point_contains, bounds_transform, Handed, Material, Mesh, Model, Sk, SkDraw, StereoKitMultiThread};
use stereokit_bevy::networking::StereoKitBevyClientPlugins;
use stereokit_bevy::{ModelBundle, ModelInfo};

#[derive(Component, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct DoNotInteract;

pub struct StereoKitInteraction;

impl Plugin for StereoKitInteraction {
    fn build(&self, app: &mut App) {
        app.add_system(hand_grab_interaction);
        app.add_system(hand_grab_interaction_2);
    }
}

#[test]
fn test() {
    let mut app = App::new();
    app.add_plugins(StereoKitBevyClientPlugins);
    app.add_plugin(StereoKitInteraction);
    app.add_startup_system(temp_start_system);
    app.run();
}

fn temp_start_system(sk: NonSend<SkDraw>, mut commands: Commands) {
    commands.spawn(ModelBundle::new(
        sk.model_create_mesh(Mesh::CUBE, Material::DEFAULT),
        ModelInfo::Cube(Vec3::splat(1.0)),
        Transform::from_scale(Vec3::splat(0.1)),
        Default::default(),
        Default::default(),
    ));
}

#[derive(Component, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TransformDifference(Transform);

fn hand_grab_interaction_2(
    sk: Res<Sk>,
    mut query: Query<(
        Entity,
        &mut Transform,
        &GlobalTransform,
        &TransformDifference,
    )>,
    mut commands: Commands,
) {
    if sk.input_hand(Handed::Right).pinch_activation < 0.5 {
        for (entity, _, _, _) in query.iter() {
            commands.entity(entity).remove::<TransformDifference>();
        }
        return;
    }
    for (_, mut transform, global_transform, transform_difference) in query.iter_mut() {
        // let temp = sk.input_hand(Handed::Right).palm.position - global_transform.translation();
        // let diff = temp - transform_difference.0.translation;
        transform.translation = sk.input_hand(Handed::Right).palm.position - transform_difference.0.translation;
        transform.rotation = sk.input_hand(Handed::Right).palm.orientation * transform_difference.0.rotation.inverse();
    }
}

fn hand_grab_interaction(
    sk: Res<Sk>,
    query: Query<(Entity, &Model, &GlobalTransform), (Without<DoNotInteract>, Without<TransformDifference>)>,
    mut commands: Commands,
) {
    for (entity, model, global_transform) in query.iter() {
        if sk.input_hand(Handed::Right).pinch_activation >= 0.5 {

            let mut bounds = sk.model_get_bounds(model);
            bounds.center = global_transform.compute_transform().transform_point(bounds.center);
            bounds.dimensions *= global_transform.compute_transform().scale;

            if bounds_point_contains(
                bounds,
                //bounds_transform(sk.model_get_bounds(model), global_transform.compute_matrix()),
                sk.input_hand(Handed::Right).palm.position,
            ) {
                let diff = Transform::from_scale(Vec3::splat(1.0))
                    .with_translation(
                        sk.input_hand(Handed::Right).palm.position - global_transform.translation(),
                    )
                    .with_rotation(
                        sk.input_hand(Handed::Right).palm.orientation *
                            global_transform.compute_transform().rotation.inverse(),
                    );
                commands.entity(entity).insert(TransformDifference(diff));
            }
        }
    }
}
