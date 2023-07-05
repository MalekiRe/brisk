use std::ops::Mul;
use openxr::{InstanceExtensions, OpenGL, PassthroughFlagsFB, Session};
use brisk::bevy_app::App;
use brisk::bevy_ecs::prelude::{Commands, NonSend};
use brisk::bevy_transform::prelude::Transform;
use brisk::glam::{Mat4, Vec3};
use brisk::leknet::LeknetClient;
use brisk::stereokit::*;
use brisk::stereokit::named_colors::BLUE_VIOLET;
use brisk::stereokit_bevy::{ModelBundle, ModelInfo};
use brisk::stereokit_bevy::networking::StereoKitBevyClientPlugins;
use brisk::stereokit_inspector::StereoKitInspector;
use brisk::stereokit_interaction::StereoKitInteraction;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on", logger(level = "debug", tag = "my-tag")))]
pub fn main() {
    println!("Hello World");
    _main();
}

pub fn _main() {
    let mut app = App::new();
    app.add_plugins(StereoKitBevyClientPlugins);
    app.add_plugin(StereoKitInspector);
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