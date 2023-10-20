pub struct BlackLevelPushUi {
    r_offset: BoundedSlider,
    gr_offset: BoundedSlider,
    gb_offset: BoundedSlider,
    b_offset: BoundedSlider,
    alpha: BoundedSlider,
    beta: BoundedSlider,
    id: usize,
}
impl BlackLevelPushUi {
    pub fn new(id: usize) -> Self {
        Self {
            r_offset: BoundedSlider {
                name: "R Offset",
                min: -100.,
                min_str: (-100.).to_string(),
                max: 100.,
                max_str: (100.).to_string(),
            },
            gr_offset: BoundedSlider {
                name: "Gr Offset",
                min: -100.,
                min_str: (-100.).to_string(),
                max: 100.,
                max_str: (100.).to_string(),
            },
            gb_offset: BoundedSlider {
                name: "Gb Offset",
                min: -100.,
                min_str: (-100.).to_string(),
                max: 100.,
                max_str: (100.).to_string(),
            },
            b_offset: BoundedSlider {
                name: "B Offset",
                min: -100.,
                min_str: (-100.).to_string(),
                max: 100.,
                max_str: (100.).to_string(),
            },
            alpha: BoundedSlider {
                name: "Alpha",
                min: -100.,
                min_str: (-100.).to_string(),
                max: 100.,
                max_str: (100.).to_string(),
            },
            beta: BoundedSlider {
                name: "Beta",
                min: -100.,
                min_str: (-100.).to_string(),
                max: 100.,
                max_str: (100.).to_string(),
            },
            id,
        }
    }
    pub fn show(&mut self, ui: &mut Ui, data: &mut BlackLevelPush) -> bool {
        let mut changed = false;
        CollapsingHeader::new("Black Level Push")
            .id_source(self.id)
            .show(ui, |ui| {
                changed |= self.r_offset.show(ui, &mut data.r_offset);
                changed |= self.gr_offset.show(ui, &mut data.gr_offset);
                changed |= self.gb_offset.show(ui, &mut data.gb_offset);
                changed |= self.b_offset.show(ui, &mut data.b_offset);
                changed |= self.alpha.show(ui, &mut data.alpha);
                changed |= self.beta.show(ui, &mut data.beta);
            });
        changed
    }
}
