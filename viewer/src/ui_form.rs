use bevy_egui::egui::{Ui, TextEdit, CollapsingHeader};

pub struct BoundedSlider{
	pub name: &'static str,
	pub min: f32,
    pub min_str: String,
	pub max: f32,
    pub max_str: String,
}

impl BoundedSlider{
    pub fn show(&mut self, ui: &mut Ui, value: &mut f32) -> bool{
        ui.spacing_mut().slider_width = 200.;
        ui.label(self.name);
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
