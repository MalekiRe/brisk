#[cfg(feature = "networking")]
pub mod networking;
#[cfg(test)]
mod tests;

use bevy_app::{App, Plugin, PluginGroup, PluginGroupBuilder};
use bevy_ecs::prelude::Bundle;
use bevy_ecs::prelude::{Component, NonSend, Query};
use bevy_transform::components::GlobalTransform;
use bevy_transform::prelude::Transform;
use bevy_transform::systems::sync_simple_transforms;
use bevy_transform::{TransformBundle, TransformPlugin};
use glam::Vec3;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use stereokit::{Color128, Model, RenderLayer, Settings, SkDraw, StereoKitDraw};

#[derive(Clone, Debug, Component)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ModelInfo {
    Mem { name: String, mem: Vec<u8> },
    Cube(Vec3),
}
impl Default for ModelInfo {
    fn default() -> Self {
        Self::Cube(Vec3::splat(1.0))
    }
}

#[cfg(not(feature = "networking"))]
pub struct StereoKitBevyPlugins;

#[cfg(not(feature = "networking"))]
impl PluginGroup for StereoKitBevyPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(StereoKitBevy)
            .add(TransformPlugin)
            .add(bevy_time::TimePlugin)
    }
}

#[cfg(not(feature = "networking"))]
pub struct StereoKitBevy;

#[cfg(not(feature = "networking"))]
impl Plugin for StereoKitBevy {
    fn build(&self, app: &mut App) {
        fn stereokit_loop(mut app: App) {
            Settings::default()
                .init()
                .unwrap()
                .run(|_| app.update(), |_| ());
        }
        app.set_runner(stereokit_loop);
        app.insert_resource(unsafe { stereokit::Sk::create_unsafe() });
        app.insert_non_send_resource(unsafe { stereokit::SkDraw::create_unsafe() });
        #[cfg(feature = "model-draw-system")]
        app.add_system(model_draw);
    }
}

#[cfg(feature = "model-draw-system")]
#[derive(Bundle)]
pub struct ModelBundle {
    model: Model,
    model_info: ModelInfo,
    transform: Transform,
    global_transform: GlobalTransform,
    color: Color128,
    render_layer: RenderLayer,
}

impl ModelBundle {
    pub fn new(
        model: Model,
        model_info: ModelInfo,
        transform: Transform,
        color: Color128,
        render_layer: RenderLayer,
    ) -> Self {
        let t = TransformBundle::from(transform);
        Self {
            model,
            model_info,
            transform: t.local,
            global_transform: t.global,
            color,
            render_layer,
        }
    }
}

#[cfg(feature = "model-draw-system")]
fn model_draw(
    query: Query<(&Model, &GlobalTransform, &Color128, &RenderLayer)>,
    sk: NonSend<SkDraw>,
) {
    query.iter().for_each(|(model, transform, color, layer)| {
        sk.model_draw(model, transform.compute_matrix(), *color, *layer)
    });
}
