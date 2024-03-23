use bevy_egui::egui::{self, TextEdit, Ui};
use glam::Mat4;

pub struct BoundedSlider {
    pub name: String,
    pub min: f32,
    pub min_str: String,
    pub max: f32,
    pub max_str: String,
}

impl BoundedSlider {
    fn new(name: String, min: f32, max: f32) -> Self {
        Self {
            name,
            min,
            min_str: min.to_string(),
            max,
            max_str: max.to_string(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui, value: &mut f32) -> bool {
        ui.spacing_mut().slider_width = 200.;
        ui.label(&self.name);
        ui.horizontal(|ui| {
            ui.label("min:");
            ui.add(TextEdit::singleline(&mut self.min_str).desired_width(50.));
            if let Ok(num) = self.min_str.parse::<f32>() {
                self.min = num;
            }
            ui.label("  max:");
            ui.add(TextEdit::singleline(&mut self.max_str).desired_width(50.));
            if let Ok(num) = self.max_str.parse::<f32>() {
                self.max = num;
            }
        });
        ui.add(bevy_egui::egui::Slider::new(value, self.min..=self.max))
            .changed()
    }
}

fn index_mat4(mat4: &mut Mat4, index: usize) -> &mut f32 {
    match index {
        0 => &mut mat4.x_axis[0],
        1 => &mut mat4.y_axis[0],
        2 => &mut mat4.z_axis[0],
        3 => &mut mat4.w_axis[0],

        4 => &mut mat4.x_axis[1],
        5 => &mut mat4.y_axis[1],
        6 => &mut mat4.z_axis[1],
        7 => &mut mat4.w_axis[1],

        8 => &mut mat4.x_axis[2],
        9 => &mut mat4.y_axis[2],
        10 => &mut mat4.z_axis[2],
        11 => &mut mat4.w_axis[2],

        12 => &mut mat4.x_axis[3],
        13 => &mut mat4.y_axis[3],
        14 => &mut mat4.z_axis[3],
        15 => &mut mat4.w_axis[3],

        _ => panic!("Invalid index"),
    }
}

pub struct Mat4Slider {
    selected: usize,
    egui_id: usize,
    sliders: [BoundedSlider; 12],
}

impl Mat4Slider {
    pub fn new(name: String, min: f32, max: f32, egui_id: usize) -> Self {
        Self {
            selected: 0,
            egui_id,
            sliders: [
                BoundedSlider::new(format!("{}_r0c0", &name), min, max),
                BoundedSlider::new(format!("{}_r0c1", &name), min, max),
                BoundedSlider::new(format!("{}_r0c2", &name), min, max),
                BoundedSlider::new(format!("{}_r0c3", &name), min, max),
                BoundedSlider::new(format!("{}_r1c0", &name), min, max),
                BoundedSlider::new(format!("{}_r1c1", &name), min, max),
                BoundedSlider::new(format!("{}_r1c2", &name), min, max),
                BoundedSlider::new(format!("{}_r1c3", &name), min, max),
                BoundedSlider::new(format!("{}_r2c0", &name), min, max),
                BoundedSlider::new(format!("{}_r2c1", &name), min, max),
                BoundedSlider::new(format!("{}_r2c2", &name), min, max),
                BoundedSlider::new(format!("{}_r2c3", &name), min, max),
            ],
        }
    }
    
    fn make_label(&mut self, index: usize, ui: &mut Ui, value: &mut Mat4){
        if ui
            .selectable_label(self.selected == index, index_mat4(value, index).to_string())
            .clicked()
        {
            self.selected = index
        };
    }

    pub fn show(&mut self, ui: &mut Ui, value: &mut Mat4) -> bool {
        egui::Grid::new(self.egui_id).show(ui, |ui| {
            for i in 0..12{
                self.make_label(i, ui, value);
                if (i + 1) % 4 == 0{
                    ui.end_row();
                }
            }
        });

        self.sliders[self.selected].show(ui, index_mat4(value, self.selected))
    }
}

pub struct IntCheckbox {
    pub name: &'static str,
}

impl IntCheckbox {
    pub fn show(&mut self, ui: &mut Ui, value: &mut i32) -> bool {
        let mut _bool = *value != 0;
        let out = ui.checkbox(&mut _bool, self.name).changed();
        *value = if _bool { 1 } else { 0 };
        out
    }
}
