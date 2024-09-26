use egui::{CentralPanel, Context};

#[derive(Default)]
pub struct AppInterface {}

impl AppInterface {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }
}

impl eframe::App for AppInterface {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui: &mut egui::Ui| {
            ui.heading("Title")
                .on_hover_text("Hello! You are hovering me!");
            ui.spacing();
            ui.label("\n");
            ui.label("I am a label!");
        });
    }
}
