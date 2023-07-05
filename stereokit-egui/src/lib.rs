use std::borrow::BorrowMut;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut, Div, Mul};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use egui::{ClippedPrimitive, Context, ecolor, emath, epaint, Event, Id, ImageData, Key, Modifiers, PaintCallback, PlatformOutput, PointerButton, Pos2, RawInput, Response, Sense, Stroke, Style, TextureId, TouchPhase, Ui, Widget};
use egui::color_picker::Alpha;
use egui::epaint::{Primitive, Shadow};
use egui::output::OutputEvent;
use egui_glow::glow::POINT;
use glam::{Mat4, Quat, Vec2, Vec3};
use glutin::config::Api;
use glutin::context::GlProfile;
use glutin::surface::Rect;
use once_cell::sync::OnceCell;
use stereokit::{ButtonState, Color128, Color32, CullMode, DepthTest, Handed, InputSource, LinePoint, Material, Mesh, MoveType, Pose, Ray, RenderLayer, Settings, Shader, SkDraw, StereoKitDraw, StereoKitMultiThread, StereoKitSingleThread, Tex, TextureFormat, TextureType, Transparency, Vert, WindowContext, WindowType};
// use stereokit::sys::mesh_create;
use zwnbsp::ZeroWidth;

#[test]
fn test() {
    main();
}
pub const POINTS_PER_METER: f32 = 1000.0;

pub struct SkEguiWindow {
    context: Context,
    textures: HashMap<egui::TextureId, (Tex, Material)>,
    last_delete: Vec<TextureId>,
    mesh_retainer: Vec<Mesh>,
    last_pos: Option<Pos2>,
}
fn convert_pos(mut pos: Pos2, size: Vec2) -> Vec3 {
    pos.x /= POINTS_PER_METER;
    pos.y /= POINTS_PER_METER;
    pos.y += size.y;
    pos.y = size.y - pos.y;
    pos.x += size.x / 2.0;
    pos.x = size.x - pos.x;
    Vec3::new(pos.x, pos.y , 0.0)
}
fn convert_pos_2(mut pos: Pos2, size: Pos2) -> Vec3 {
    pos.y += size.y;
    pos.y = size.y - pos.y;
    pos.x += size.x / 2.0;
    pos.x = size.x - pos.x;

    pos.x /= POINTS_PER_METER;
    pos.y /= POINTS_PER_METER;

    Vec3::new(pos.x, pos.y, 0.0)

}
impl SkEguiWindow {
    pub fn run(&mut self, ui: &WindowContext, sk: &SkDraw, size: Vec2, content_closure: impl FnOnce(&Context) + Sized) {
        let scale = Vec3::new(-0.001, -0.001, -0.01);
        let offset = Vec3::new(size.x / 2.0, 0.0, -0.0001);
        let context = self.context.clone();
        let full_output = context.run(self.gather_raw_input(sk, size, scale, offset), content_closure);
        let clipped_primitives = context.tessellate(full_output.shapes);
        for (id, mut image_delta) in full_output.textures_delta.set {
            self.set_texture(id, &mut image_delta, sk);
        }
        self.paint(ui, sk, clipped_primitives, offset, scale, size);
        self.handle_platform_output(sk, full_output.platform_output);
        self.last_delete = full_output.textures_delta.free;
    }
    fn gather_raw_input(&mut self, sk: &SkDraw, size: Vec2, scale: Vec3, offset: Vec3) -> RawInput {
        let pose = sk.hierarchy_to_world_pose(Pose::new(offset, Quat::IDENTITY));
        let model_transform_matrix = Mat4::from_scale_rotation_translation(scale, pose.orientation, pose.position);
        let mat = model_transform_matrix.inverse();
        let mut raw_input = RawInput::default();
        raw_input.pixels_per_point = Some(4.0);
        raw_input.screen_rect = Some(egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(POINTS_PER_METER * size.x, POINTS_PER_METER * size.y)));
        for mesh in &self.mesh_retainer {
            let pointer_indices = sk.input_pointer_count(InputSource::HAND_RIGHT);
            let mut break_flag = false;
            for i in 0..pointer_indices {
                let pointer = sk.input_pointer(i, InputSource::HAND_RIGHT);
                let mut ray = pointer.ray;
                let ray = Ray {
                    pos: mat.transform_point3(ray.pos.into()).into(),
                    dir: mat.transform_vector3(ray.dir.into()).into(),
                };
                if let Some((mut ray, _)) = sk.mesh_ray_intersect(mesh, ray, CullMode::None) {
                    break_flag = true;

                    ray.pos.x *= scale.x.abs();
                    ray.pos.y *= scale.y.abs();

                    let pos = Pos2::new(ray.pos.x * POINTS_PER_METER, ray.pos.y *  POINTS_PER_METER);
                    raw_input.events.push(Event::PointerMoved(pos));


                    let transfomred_hand_pos = mat.transform_point3(sk.input_hand(Handed::Right).pinch_pt).z;

                    raw_input.events.push(Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed: transfomred_hand_pos <= 5.0,
                        modifiers: Default::default(),
                    });
                    self.last_pos.replace(pos);
                } else {
                    if let Some(pos) = self.last_pos.take() {
                        raw_input.events.push(
                            Event::PointerButton {
                                pos,
                                button: PointerButton::Primary,
                                pressed: false,
                                modifiers: Default::default(),
                            }
                        )
                    } else {
                        raw_input.events.push(Event::PointerGone)
                    }
                }
            }
            if break_flag {
                break;
            }
        }
        let mut char = sk.input_text_consume();
        let mut text_buffer = String::new();
        while char != '\0' {
            if !char.is_control() {
                text_buffer.push(char);
                char = sk.input_text_consume();
                continue;
            }

            let mut modifiers = Modifiers::NONE;

            if sk.input_key(stereokit::Key::Shift).contains(ButtonState::ACTIVE) {
                modifiers = modifiers.plus(Modifiers::SHIFT);
            }
            if sk.input_key(stereokit::Key::Ctrl).contains(ButtonState::ACTIVE) {
                modifiers = modifiers.plus(Modifiers::CTRL)
            }

            match char {
                '\u{8}' => {
                    raw_input.events.push(Event::Key {
                        key: Key::Backspace,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    });
                    // if idx == 0 {
                    //     break;
                    // }
                    /*rope.remove(idx - 1..idx);
                    match text_cursor.char {
                        0 => { text_cursor.line -= 1; /*TODO something with char */ }
                        _ => text_cursor.char -= 1
                    }*/
                }
                '\r' => {
                    raw_input.events.push(Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    })
                    /*rope.insert_char(idx, '\n');
                    text_cursor.line += 1;
                    text_cursor.char = 0;*/
                }
                '\0' => {}
                _ => {
                    println!("{:#?}", char);
                }
            }
            char = sk.input_text_consume();
        }
        raw_input.events.push(Event::Text(text_buffer));
        raw_input
    }
    fn handle_platform_output(&mut self, sk: &SkDraw, output: PlatformOutput) {
        for x in output.events.iter() {
            match x {
                OutputEvent::Clicked(_) => {
                }
                OutputEvent::DoubleClicked(_) => {}
                OutputEvent::TripleClicked(_) => {}
                OutputEvent::FocusGained(_) => {}
                OutputEvent::TextSelectionChanged(_) => {}
                OutputEvent::ValueChanged(_) => {}
            }
        }
    }
    fn paint(&mut self, ui: &WindowContext, sk: &SkDraw, mut primitives: Vec<ClippedPrimitive>, offset: Vec3, scale: Vec3, size: Vec2) {
        self.mesh_retainer.clear();
        for _ in 0..primitives.len() {
            self.mesh_retainer.push(sk.mesh_create());
        }
        for (i, primitive) in primitives.into_iter().enumerate() {
            match primitive {
                ClippedPrimitive { clip_rect, primitive } => {
                    match primitive {
                        Primitive::Mesh(mesh) => {
                            if !self.textures.contains_key(&mesh.texture_id) {
                                continue;
                            }
                            let m = self.mesh_retainer.get_mut(i).unwrap();
                            let mut verts = vec![];
                            for vertex in mesh.vertices {
                                verts.push(Vert {
                                    pos: Vec3::new(vertex.pos.x, vertex.pos.y, 0.0),
                                    norm: Vec3::new(0.0, 0.0, -1.0),
                                    uv: Vec2::new(vertex.uv.x, vertex.uv.y),
                                    col: Color32::new(vertex.color.r(), vertex.color.g(), vertex.color.b(), vertex.color.a()),
                                })
                            }
                            sk.mesh_set_data(&m, &verts, &mesh.indices, true);
                            if let Some((texture, material)) = self.textures.get(&mesh.texture_id) {
                                sk.material_set_texture(material, "diffuse", texture);
                                sk.material_set_cull(material, CullMode::None);
                                sk.material_set_depth_test(material, DepthTest::LessOrEq);
                                sk.material_set_depth_write(material, false);
                                sk.material_set_queue_offset(material, 200);
                                let matrix = Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, offset);
                                sk.mesh_draw(m, material, matrix, Color128::new_rgb(1.0, 1.0, 1.0), RenderLayer::LAYER_ALL);
                            }
                        }
                        Primitive::Callback(a) => {
                            match a {
                                PaintCallback { rect, callback } => {
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    fn set_texture(&mut self, tex_id: egui::TextureId, delta: &mut egui::epaint::ImageDelta, sk: &SkDraw) {
        if self.textures.contains_key(&tex_id) {
            return;
        }
        let (texture, _) = self.textures.entry(tex_id).or_insert_with(|| {
            (sk.tex_create(TextureType::IMAGE_NO_MIPS, TextureFormat::RGBA32), sk.material_copy(Material::UNLIT_CLIP))
        });
        match &mut delta.image {
            ImageData::Color(image) => {
                let mut data: Vec<_> = image.pixels.iter().map(|c| Color32::new(c.r(), c.g(), c.b(), c.a())).collect();
                unsafe {
                    stereokit::sys::tex_set_colors(texture.0.as_ptr(), image.size[0] as i32, image.size[1] as i32, data.as_mut_ptr() as *mut c_void);
                }
            }
            ImageData::Font(image) => {
                let mut data: Vec<Color32> = image
                    .srgba_pixels(None)
                    .into_iter().map(|a| Color32::new(a.r(), a.g(), a.b(), a.a()))
                    .collect();
                unsafe {
                    stereokit::sys::tex_set_colors(texture.0.as_ptr(), image.size[0] as i32, image.size[1] as i32, data.as_mut_ptr() as *mut c_void);
                }
            }
        }
    }
}
impl Default for SkEguiWindow {
    fn default() -> Self {
        Self {
            context: Context::default(),
            textures: HashMap::default(),
            last_delete: vec![],
            mesh_retainer: vec![],
            last_pos: None,
        }
    }
}

pub fn get_sk_egui_window(id: impl AsRef<str>) -> &'static mut SkEguiWindow {
    static mut INSTANCE: OnceCell<Mutex<HashMap<String, SkEguiWindow>>> = OnceCell::new();
    let map = unsafe {
        INSTANCE.get_or_init( || Mutex::new(HashMap::new()));
        INSTANCE.get_mut().unwrap().get_mut().unwrap() };
    let id = id.as_ref();
    if !map.contains_key(id) {
        println!("doens't contain key! adding!");
        map.insert(id.to_string(), SkEguiWindow::default());
    }
    map.get_mut(id).unwrap()
}

pub trait SkEguiWindowTrait {
    fn egui_window<S: AsRef<str>>(&self, window_title: S, pose: impl AsMut<Pose>, size: impl Into<Vec2> + Clone, window_type: WindowType, move_type: MoveType, content_closure: impl FnOnce(&Context));
}
impl SkEguiWindowTrait for SkDraw {
    fn egui_window<S: AsRef<str>>(&self, window_title: S, pose: impl AsMut<Pose>, size: impl Into<Vec2> + Clone, window_type: WindowType, move_type: MoveType, content_closure: impl FnOnce(&Context)) {
        let size_stored = size.clone();
        let window_title = window_title.as_ref();
        self.window(window_title, pose, size, window_type, move_type, |ui| {
            get_sk_egui_window(window_title).run(ui, self, size_stored.into(), content_closure);
        });
    }
}

pub trait SkEguiUi {
    fn sk_button(&mut self, text: impl AsRef<str>) -> Response;
}

impl SkEguiUi for Ui {
    fn sk_button(&mut self, text: impl AsRef<str>) -> Response {
        self.add(SkButton::new(text.as_ref()))
    }
}

pub struct SkButton {
    text: String
}
impl SkButton {
    pub fn new(text: impl AsRef<str>) -> Self {
        Self {
            text: text.as_ref().to_string(),
        }
    }
}
impl Widget for SkButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let c_str = CString::new(self.text.clone()).unwrap();
        let text_size: Vec2 = unsafe {
            stereokit::sys::text_size(c_str.as_ptr(), stereokit::sys::ui_get_text_style())
        }.into();
        let text_size = text_size.div(Vec2::splat(POINTS_PER_METER)).mul(Vec2::splat(3.0));
        let mut button_pos = ui.next_widget_position();
        let (rect, mut response) = ui.allocate_exact_size(egui::Vec2::new(text_size.x, text_size.y), egui::Sense::click());
        ui.add_space(text_size.y + 30.0);
        let window = unsafe { WindowContext::create_unsafe()};
        let mut screen_rect = None;
        ui.painter().add(egui::PaintCallback {
            rect,
            callback: Arc::new(()),
        });
        ui.input(|state| {
           screen_rect.replace(state.screen_rect)
        });
        let screen_rect = screen_rect.unwrap();
        let rel_pos = convert_pos_2(button_pos, Pos2::new(screen_rect.width(), screen_rect.height()));
        let button_size = Vec2::new(rect.width() * POINTS_PER_METER, rect.height() * POINTS_PER_METER);
        let click = window.button_at(self.text.clone(), rel_pos, button_size);
        response.sense = Sense {
            click,
            drag: false,
            focusable: true,
        };
        response.clicked[0] = click;
        response
    }
}
//
// static SHADER_FILE: &[u8]= skshader_macro::my_macro!(
//     //--name                 = unlit/test
//     //--time: color          = 1
//     //--tex: 2D              = white
//     //--uv_scale: range(0,2) = 0.5
//     //--chunks = 1, 2, 2, 1
//
//     // This is for the system to load in global values
//     cbuffer SystemBuffer : register(b1) {
//         float4x4 viewproj;
//     };
//
//     // And these are for instanced rendering
//     struct Inst {
//         float4x4 world;
//     };
//     cbuffer TransformBuffer : register(b2) {
//         Inst inst[100];
//     };
//
//     /* Ugh */
//
//     /*struct vsIn {
//         float4 pos  : SV_POSITION;
//         float3 norm : NORMAL;
//         float2 uv   : TEXCOORD0;
//         float4 color: COLOR0;
//     };*/
//
//     struct vsIn {
//         float4 pos  : SV_POSITION;
//         float3 norm : NORMAL;
//         float2 uv   : TEXCOORD0;
//         float4 color: COLOR0;
//     };
//     struct psIn {
//         float4 pos   : SV_POSITION;
//         float2 uv    : TEXCOORD0;
//         float3 color : COLOR0;
//     };
//
//     uint chunks[4];
//     float tex_scale;
//     float4 time;
//
//     Texture2D    tex         : register(t0);
//     SamplerState tex_sampler : register(s0);
//
//     psIn vs(vsIn input, uint id : SV_InstanceID) {
//         psIn output;
//         output.pos = mul(float4(input.pos.xyz, 1), inst[id].world);
//         output.pos = mul(output.pos, viewproj);
//         float3 normal = normalize(mul(float4(input.norm, 0), inst[id].world).xyz);
//         output.color = saturate(dot(normal, float3(0,1,0))).xxx * input.color.rgb;
//         output.uv = input.uv * tex_scale * time.x;
//         return output;
//     }
//     float4 ps(psIn input) : SV_TARGET {
//         return float4(input.color, 1) * tex.Sample(tex_sampler, input.uv);
//     }
// );

pub fn main() {
    let sk = Settings::default().init().unwrap();
    let mut color = ecolor::Color32::default();
    let mut pose = Pose::IDENTITY;
    sk.run(|sk| {
        /*
        sk.window("hi", &mut pose, [0.1, 0.1], WindowType::Normal, MoveType::Exact, |ui| {
           ui.button2("hi");
           ui.button2("hi");
           ui.button("hello");
            ui.button("hello");
            let time = Instant::now();
            for i in 0..200000 {
                ui.idi(i, |ui, _| {
                    ui.button2("hi");
                });
            }
            let leftover = Instant::now() - time;
            println!("leftover: {:?}", leftover);
        });
        return;
        sk.egui_window("my window", &mut pose, [0.8, 0.8], WindowType::Normal, MoveType::Exact, |ctx| {
            egui::CentralPanel::default().show(&ctx, |ui| {
                ui.label("Hello world!");
                if ui.button("Click me").is_pointer_button_down_on() {
                    ui.label("i'm clicked!");
                }
                ui.collapsing("some menu", |ui| {
                   ui.label("hi");
                    ui.label("hello");
                });
                ui.visuals_mut().override_text_color = Some(egui::Color32::GREEN);
                let mut pos = Pos2::default();
                ui.input(|i| {
                   if let Some(a) = i.pointer.interact_pos() {
                       pos = a;
                   }
                });
                ui.painter().circle(pos, 1.0, egui::Color32::GREEN, Stroke::new(1.0, egui::Color32::GREEN));
                if ui.sk_button("hi").clicked() {
                   println!("hi");
                }
                ui.label("hey");
                egui::widgets::color_picker::color_picker_color32(ui, &mut color, Alpha::Opaque)
            });
        });

         */
    }, |sk| {});
}

trait TestExtra {
    #[track_caller]
    fn button2(&self, text: &str) -> bool;
}
impl TestExtra for WindowContext {
    fn button2(&self, text: &str) -> bool {
        let location: &'static std::panic::Location<'static> = std::panic::Location::caller();
        let id = self.push_idi((location.line() * location.column()) as i32);
        if unsafe { self.get_locations().contains(&id)} {
            panic!("StereoKit Ui Error: same function was called in the same location multiple times without a different hash");
        }
        unsafe {
            self.get_locations().insert(id);
        }
        let ret = self.button(text);
        self.pop_id();
        ret
    }
}