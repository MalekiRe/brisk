use crate::ModelInfo;
use bevy_ecs::prelude::{Commands, Res};
use bevy_transform::prelude::Transform;
use glam::{Mat4, Quat, Vec3};
use stereokit::{Color128, Material, Mesh, RenderLayer, Sk, StereoKitDraw, StereoKitMultiThread};

fn add_example_model(mut commands: Commands, sk: Res<Sk>) {
    commands.spawn(crate::ModelBundle::new(
        sk.model_create_mesh(stereokit::Mesh::CUBE, stereokit::Material::DEFAULT),
        ModelInfo::Cube(Vec3::splat(1.0)),
        Transform::from_translation(Vec3::new(0.1, 0.0, 0.0)),
        Color128::new(0.1, 0.4, 0.0, 0.6),
        RenderLayer::LAYER1,
    ));
}

#[test]
#[cfg(not(feature = "networking"))]
fn run_plugin_itself() {
    bevy_app::App::new()
        .add_plugins(crate::StereoKitBevyPlugins)
        .add_startup_system(add_example_model)
        .run();
}

#[test]
#[cfg(not(feature = "networking"))]
fn stereokit_with_bevy() {
    let sk = stereokit::Settings::default().init().unwrap();
    let mut app = bevy_app::App::new();
    app.add_plugin(crate::StereoKitBevy);
    app.add_startup_system(add_example_model);

    let second_model = sk.model_create_mesh(Mesh::SPHERE, Material::PBR);

    let pos = Mat4::from_scale_rotation_translation(
        Vec3::new(1.0, 1.0, 1.0),
        Quat::IDENTITY,
        Vec3::new(0.0, 0.0, 1.0),
    );

    sk.run(
        |sk| {
            sk.model_draw(
                &second_model,
                pos,
                Color128::new(0.4, 0.0, 0.2, 0.5),
                RenderLayer::LAYER_ALL,
            );
            app.update();
        },
        |_| {},
    );
}
