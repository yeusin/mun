use eframe::egui;
fn test(v: &egui::Visuals) {
    let _ = v.panel_fill.to_normalized_gamma_f32();
}
