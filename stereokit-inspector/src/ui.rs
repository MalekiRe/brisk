use bevy_reflect::FromReflect;
use image::{ImageBuffer, Rgb};
use stereokit::{Color128, Color32, WindowContext};


///Implement this on components you want to do custom display for
pub trait InspectorDisplay {
    fn draw(&mut self, ui: &WindowContext);
}