use eframe::egui;
struct MyApp;
impl eframe::App for MyApp {
    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {}
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }
}
fn main() {}
