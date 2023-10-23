use bevy_egui::egui::{Ui, TextEdit};
use glam::Mat4;

pub struct BoundedSlider{
	pub name: String,
	pub min: f32,
    pub min_str: String,
	pub max: f32,
    pub max_str: String,
}

impl BoundedSlider{
    pub fn show(&mut self, ui: &mut Ui, value: &mut f32) -> bool{
        ui.spacing_mut().slider_width = 200.;
        ui.label(&self.name);
        ui.horizontal(|ui|{
            ui.label("min:");
            ui.add(TextEdit::singleline(&mut self.min_str).desired_width(50.));
            if let Ok(num) = self.min_str.parse::<f32>(){
                self.min = num;
            }
            ui.label("  max:");
            ui.add(TextEdit::singleline(&mut self.max_str).desired_width(50.));
            if let Ok(num) = self.max_str.parse::<f32>(){
                self.max = num;
            }
        });
        ui.add(bevy_egui::egui::Slider::new(value, self.min..=self.max)).changed()
    }
}

pub struct Mat4Slider([BoundedSlider; 16]);

impl Mat4Slider{
    pub fn new(name: String, min: f32, max: f32) -> Self{
        Self([
            BoundedSlider{
                name: format!("{}_r0c0", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r0c1", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r0c2", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r0c3", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            
            BoundedSlider{
                name: format!("{}_r1c0", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r1c1", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r1c2", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r1c3", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            
            BoundedSlider{
                name: format!("{}_r2c0", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r2c1", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r2c2", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r2c3", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            
            BoundedSlider{
                name: format!("{}_r3c0", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r3c1", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r3c2", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
            BoundedSlider{
                name: format!("{}_r3c3", &name),
                min,
                min_str: min.to_string(),
                max,
                max_str: max.to_string(),
            },
        ])
    }

    pub fn show(&mut self, ui: &mut Ui, value: &mut Mat4) -> bool{
        let mut changed = false;
        changed |= self.0[0].show(ui, &mut value.x_axis[0]);
        changed |= self.0[1].show(ui, &mut value.y_axis[0]);
        changed |= self.0[2].show(ui, &mut value.z_axis[0]);
        changed |= self.0[3].show(ui, &mut value.w_axis[0]);

        changed |= self.0[4].show(ui, &mut value.x_axis[1]);
        changed |= self.0[5].show(ui, &mut value.y_axis[1]);
        changed |= self.0[6].show(ui, &mut value.z_axis[1]);
        changed |= self.0[7].show(ui, &mut value.w_axis[1]);

        changed |= self.0[8].show(ui, &mut value.x_axis[2]);
        changed |= self.0[9].show(ui, &mut value.y_axis[2]);
        changed |= self.0[10].show(ui, &mut value.z_axis[2]);
        changed |= self.0[11].show(ui, &mut value.w_axis[2]);

        changed |= self.0[12].show(ui, &mut value.x_axis[3]);
        changed |= self.0[13].show(ui, &mut value.y_axis[3]);
        changed |= self.0[14].show(ui, &mut value.z_axis[3]);
        changed |= self.0[15].show(ui, &mut value.w_axis[3]);
        changed
    }
}


pub struct IntCheckbox{
	pub name: &'static str,
}

impl IntCheckbox{
    pub fn show(&mut self, ui: &mut Ui, value: &mut i32) -> bool{
        let mut _bool = *value != 0;
        let out = ui.checkbox(&mut _bool, self.name).changed();
        *value = if _bool { 1 } else { 0 };
        out
    }
}
