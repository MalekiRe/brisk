mod field_access;
mod ui;

use std::any::{Any, TypeId};
use std::borrow::BorrowMut;
use std::collections::{HashMap, HashSet};
use std::f32::{INFINITY, NEG_INFINITY};
use std::ops::RangeInclusive;
use bevy_app::{App, AppTypeRegistry, Plugin};
use bevy_ecs::archetype::Archetype;
use bevy_ecs::component::{Component, ComponentId};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Commands, Mut, NonSend, ReflectComponent, Res, Without, World};
use bevy_ecs::reflect::ReflectComponentFns;
use bevy_ecs::system::Resource;
use bevy_ecs::world::EntityRef;
use bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy_hierarchy::Parent;
use bevy_reflect::{FromReflect, GetField, NamedField, Reflect, reflect_trait, ReflectMut, ReflectRef, TypeInfo};
use bevy_transform::prelude::Transform;
use stereokit::{Color128, Color32, Material, Mesh, Model, MoveType, Pose, Shader, Sk, SkDraw, StereoKitMultiThread, Transparency, UiColor, UiCut, WindowContext, WindowType};
use stereokit_bevy::{ModelBundle};
use const_random::const_random;
use egui::{Checkbox, CollapsingHeader, DragValue, ecolor, Slider, Ui, Widget, widgets};
use egui::color_picker::Alpha;
use glam::{EulerRot, Quat, Vec3};
use stereokit_egui::SkEguiWindowTrait;
use crate::field_access::FieldAccess;


fn main() {
    let mut app = App::new();
    //app.add_plugins(StereoKitBevyMinimalPlugins);
    app.add_plugin(StereoKitInspector);
    app.add_startup_system(test_spawn);
    app.run();
}

fn test_spawn(mut commands: Commands, sk: Res<Sk>) {
    let mut entity = commands.spawn(
        ModelBundle::new(sk.model_create_mesh(Mesh::SPHERE, Material::DEFAULT),
                         Default::default(),
                         Transform::from_scale([0.1, 0.1, 0.04].into()),
                         Color128::new_rgb(1.0, 0.0, 0.1),
                         Default::default()));
    entity.insert(ExampleStruct::default());
    for i in 0..4 {
        commands.spawn(
            ModelBundle::new(sk.model_create_mesh(Mesh::SPHERE, Material::DEFAULT),
                             Default::default(),
                             Transform::from_scale([0.1, 0.1, 0.04].into()).with_translation(Vec3::splat(i as f32 / 2.0)),
                             Color128::new_rgb(1.0, 0.0, 0.1),
                             Default::default()));
    }
}

#[derive(Default, Component, Reflect, FromReflect)]
#[reflect(Component)]
pub struct ExampleStruct {
    field1: ExampleField1,
    field2: ExampleField2,
    field3: MyEnum,
}

#[derive(Reflect, FromReflect)]
pub enum MyEnum {
    Variant1(String),
    Variant2,
}

impl Default for MyEnum {
    fn default() -> Self {
        MyEnum::Variant1(String::from("hi"))
    }
}

#[derive(Reflect, FromReflect)]
pub struct ExampleField1 {
    x: f32,
    y: f32,
    z: f32,
}
impl Default for ExampleField1 {
    fn default() -> Self {
        Self {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        }
    }
}
#[derive(Reflect, FromReflect)]
pub struct ExampleField2 {
    a: String,
    b: bool,
    c: char,
}
impl Default for ExampleField2 {
    fn default() -> Self {
        Self {
            a: "Hi Mom!".to_string(),
            b: true,
            c: 'E',
        }
    }
}

pub trait DisplayComponent: Reflect {
    fn display_component(&mut self, ui: &mut Ui);
}

impl DisplayComponent for Color128 {
    fn display_component(&mut self, ui: &mut Ui) {
        CollapsingHeader::new("Color").default_open(true).show(ui, |ui| {
            let color: Color32 = (*self).into();
            let mut color = ecolor::Color32::from_rgba_premultiplied(color.r, color.g, color.b, color.a);
            egui::color_picker::color_picker_color32(ui, &mut color, Alpha::BlendOrAdditive);
            let color = Color32::new(color.r(), color.g(), color.b(), color.a());
            *self = color.into();
        });
    }
}

impl DisplayComponent for Model {
    fn display_component(&mut self, ui: &mut Ui) {
        let sk = unsafe { Sk::create_unsafe()};
        ui.collapsing("Model", |ui| {
            let id = sk.asset_get_id(self);
            ui.label(id);
        });
    }
}

impl DisplayComponent for Transform {
    fn display_component(&mut self, ui: &mut Ui) {
        const DRAG_SPEED: f32 = 0.03;
        const ROTATION_DRAG_SPEED: f32 = 0.5;
        ui.collapsing("Transform", |ui| {
            ui.horizontal(|ui| {
                ui.label("Pos:");
                ui.add_space(ui.spacing().indent * 3.0);
                DragValue::new(&mut self.translation.x).speed(DRAG_SPEED).prefix("x: ").ui(ui);
                DragValue::new(&mut self.translation.y).speed(DRAG_SPEED).prefix("y: ").ui(ui);
                DragValue::new(&mut self.translation.z).speed(DRAG_SPEED).prefix("z: ").ui(ui);
            });
            ui.horizontal(|ui| {
                ui.label("Rot:");
                ui.add_space(ui.spacing().indent * 3.0);
                let mut rotation = self.rotation.to_euler(EulerRot::XYZ);
                rotation.0 = rotation.0.to_degrees();
                rotation.1 = rotation.1.to_degrees();
                rotation.2 = rotation.2.to_degrees();
                DragValue::new(&mut rotation.0).speed(ROTATION_DRAG_SPEED).prefix("x: ").ui(ui);
                DragValue::new(&mut rotation.1).speed(ROTATION_DRAG_SPEED).prefix("y: ").ui(ui);
                DragValue::new(&mut rotation.2).speed(ROTATION_DRAG_SPEED).prefix("z: ").ui(ui);
                self.rotation = Quat::from_euler(EulerRot::XYZ, rotation.0.to_radians(), rotation.1.to_radians(), rotation.2.to_radians());
            });
            ui.horizontal(|ui| {
                ui.label("Scale:");
                ui.add_space(ui.spacing().indent * 3.0);
                DragValue::new(&mut self.scale.x).speed(DRAG_SPEED).prefix("x: ").ui(ui);
                DragValue::new(&mut self.scale.y).speed(DRAG_SPEED).prefix("y: ").ui(ui);
                DragValue::new(&mut self.scale.z).speed(DRAG_SPEED).prefix("z: ").ui(ui);
            });
        });
    }
}


pub struct DisplayComponentList(pub Vec<Box<dyn Fn(&mut dyn Reflect, &mut Ui) -> bool>>);

impl DisplayComponentList {
    pub fn display(&self, component: &mut dyn Reflect, ui: &mut Ui) {
        for func in self.0.iter() {
            if func(component, ui) {
                return;
            }
        }
        let name = component.type_name().to_string();
        ui.collapsing(name, |ui| {
            Self::recursively_show_fields(component, ui);
        });
    }
    pub fn new() -> Self {
        let mut this = Self { 0: vec![] };
        this.init();
        this
    }
    fn init(&mut self) {
        self.push(|reflect, ui| {
            if let Some(color128) = reflect.downcast_mut::<Color128>() {
                color128.display_component(ui);
                return true
            }
            false
        });
        self.push(|reflect, ui| {
           match reflect.downcast_mut::<i32>() {
               None => false,
               Some(val) => {
                   ui.label(format!("{}", val));
                   true
               }
           }
        });
        self.push(|reflect, ui| {
           match reflect.downcast_mut::<Transform>() {
               None => false,
               Some(val) => {
                   val.display_component(ui);
                   true
               }
           }
        });
        self.push(|reflect, ui| {
           match reflect.downcast_mut::<Model>() {
               None => false,
               Some(val) => {
                   val.display_component(ui);
                   true
               }
           }
        });
    }
    fn push(&mut self, function: impl Fn(&mut dyn Reflect, &mut Ui) -> bool + 'static) {
        self.0.push(Box::new(function));
    }
    fn recursively_show_fields(field: &mut dyn Reflect, ui: &mut Ui) {
        if let Some(val) = field.downcast_mut::<i32>() {
            DragValue::new(val).ui(ui);
            // ui.label(format!("{val}"));
        }
        else if let Some(val) = field.downcast_mut::<f32>() {
            DragValue::new(val).ui(ui);
            //ui.label(format!("{val}"));
        }
        else if let Some(val) = field.downcast_mut::<bool>() {
            Checkbox::without_text(val).ui(ui);
            //ui.label(format!("{val}"));
        }
        else if let Some(val) = field.downcast_mut::<String>() {
            ui.text_edit_multiline(val);
            //ui.label(format!("{val}"));
        } else {
            match field.reflect_mut() {
                ReflectMut::Struct(strct) => {
                    for i in 0..strct.field_len() {
                        let name = strct.name_at(i).unwrap().to_string();
                        ui.collapsing(name, |ui| {
                            let field = strct.field_at_mut(i).unwrap();
                            Self::recursively_show_fields(field, ui);
                        });
                    }
                }
                ReflectMut::TupleStruct(_) => {}
                ReflectMut::Tuple(_) => {}
                ReflectMut::List(_) => {}
                ReflectMut::Array(_) => {}
                ReflectMut::Map(_) => {}
                ReflectMut::Enum(enm) => {
                    ui.label(enm.variant_name());
                }
                ReflectMut::Value(_) => {}
            }
        }
    }
}

#[derive(Resource)]
pub struct InspectorWindow {
    drop_down_button: bool,
    color_temp: Color128,
    pose: Pose,
    entity_to_show: Option<Entity>,
}

impl Default for InspectorWindow {
    fn default() -> Self {
        Self {
            drop_down_button: false,
            color_temp: Default::default(),
            pose: Pose::IDENTITY,
            entity_to_show: None,
        }
    }
}

fn show_components(world: &mut World) {
    let sk = unsafe { stereokit::SkDraw::create_unsafe()};
    let mut inspector_window = world.remove_resource::<InspectorWindow>().unwrap();
    let mut display_components_list = DisplayComponentList::new();
    sk.egui_window("Inspector", &mut inspector_window.pose, [0.4, 0.8], WindowType::Normal, MoveType::Exact, |ctx| {
        let mut entity_ids = vec![];
        for entity in  world.iter_entities() {
            entity_ids.push(entity.id());
        }
        egui::SidePanel::left("Entities").show(ctx, |ui| {
            for entity in entity_ids {
                if ui.button(format!("Entity: {}", entity.index())).clicked() {
                    inspector_window.entity_to_show.replace(entity);
                }
            }
        });
        egui::SidePanel::right("Components").show(ctx, |ui| {
            if let Some(entity) = inspector_window.entity_to_show {
                let mut entity_components = EntityComponents::from_entity(world, entity);
                for component in &entity_components.components {
                    if let Some(refl) = get_reflect_impl(world, component) {
                        if let Some(mut repr) = refl.reflect_mut(world.entity_mut(entity).borrow_mut()) {
                            display_components_list.display(repr.as_reflect_mut(), ui);
                        }
                    }
                }
            }
        });
    });
    world.insert_resource(inspector_window);
}

#[derive(Default, Resource)]
struct EntityTracker {
    tracked: HashSet<Entity>,
}

#[derive(Component)]
struct TrackedInSpyglass;

pub struct StereoKitInspector;
impl Plugin for StereoKitInspector {
    fn build(&self, app: &mut App) {
        app.register_type::<stereokit::Color128>();
        app.register_type::<Transform>();
        app.register_type::<f32>();
        app.register_type::<f64>();
        app.register_type::<i8>();
        app.register_type::<u8>();
        app.register_type::<i16>();
        app.register_type::<u16>();
        app.register_type::<i32>();
        app.register_type::<u32>();
        app.register_type::<i64>();
        app.register_type::<u64>();
        app.register_type::<isize>();
        app.register_type::<usize>();
        app.register_type::<ExampleStruct>();
        app.register_type::<Model>();
        app.add_system(show_components);
        app.insert_resource(InspectorWindow::default());
    }
}

fn get_reflect_impl(world: &World, name: &str) -> Option<ReflectComponent> {
    let registry = world.get_resource::<AppTypeRegistry>().unwrap().read();
    let registration = registry.get_with_name(name)?;
    registration.data::<ReflectComponent>().cloned()
}

#[derive(Debug)]
struct EntityComponents {
    components: Vec<String>,
    reprs: HashMap<String, Box<dyn Reflect>>,
}


impl EntityComponents {
    fn from_entity(world: &World, entity: Entity) -> Self {
        let loc = world.entities().get(entity).unwrap();
        let archetype = world.archetypes().get(loc.archetype_id).unwrap();
        let mut components = vec![];
        let mut reprs = HashMap::default();
        for comp in archetype.components() {
            let name = if let Some(name) = world.components().get_name(comp) {
                if let Some(refl) = get_reflect_impl(world, name) {
                    if let Some(repr) = refl.reflect(world.entity(entity)) {
                        reprs.insert(name.to_string(), repr.clone_value());
                    }
                }
                name.to_string()
            } else if let Some(id) = world.components().get_info(comp).map(|info| info.type_id()) {
                format!("TypeId({id:?}")
            } else {
                format!("ComponentId({comp:?})")
            };

            components.push(name);
        }
        components.sort_unstable();
        Self { components, reprs }
    }
}