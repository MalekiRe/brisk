use crate::{ModelBundle, ModelInfo};
use bevy_ecs::prelude::{Commands, Component, NonSend, Query, Res};
use bevy_transform::prelude::Transform;
use glam::Vec3;
use leknet::Networked;
use stereokit::{Handed, Material, Sk, SkDraw, StereoKitMultiThread};

#[test]
fn server_test() {
    let mut app = bevy_app::App::new();
    app.add_plugins(crate::networking::StereoKitBevyServerPlugins);
    app.add_startup_system(leknet::start_server);
    app.run();
}

#[test]
fn client_test() {
    let mut app = bevy_app::App::new();
    app.add_plugins(crate::networking::StereoKitBevyClientPlugins);
    app.add_startup_system(leknet::connect_to_server);
    app.add_startup_system(add_example_model);
    app.add_system(sync_example_model);
    app.run();
}

#[derive(Component)]
struct RightHand;

fn add_example_model(mut commands: Commands, sk: NonSend<SkDraw>) {
    let model_bundle = ModelBundle::new(
        sk.model_create_mesh(sk.mesh_gen_cube(Vec3::splat(0.1), 1), Material::DEFAULT),
        ModelInfo::Cube(Vec3::splat(0.1)),
        Default::default(),
        stereokit::named_colors::AQUAMARINE,
        Default::default(),
    );
    commands
        .spawn(model_bundle)
        .insert(RightHand)
        .insert(Networked);
}

fn sync_example_model(sk: Res<Sk>, mut query: Query<(&RightHand, &mut Transform)>) {
    for (_, mut transform) in query.iter_mut() {
        let palm = sk.input_hand(Handed::Right).palm;
        let temp = transform
            .with_rotation(palm.orientation)
            .with_translation(palm.position);
        transform.translation = temp.translation;
        transform.rotation = temp.rotation;
    }
}
