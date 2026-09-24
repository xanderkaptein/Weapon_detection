use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Weapon Detection",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

#[derive(Default)]
struct MyApp {
    selected_file: Option<String>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Weapon Detection");

            ui.separator();

            if ui.button("Afbeelding openen").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Afbeeldingen", &["png", "jpg", "jpeg"])
                    .pick_file()
                {
                    self.selected_file = Some(file.display().to_string());
                }
            }

            if ui.button("Video openen").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Video's", &["mp4", "avi", "mov"])
                    .pick_file()
                {
                    self.selected_file = Some(file.display().to_string());
                }
            }

            ui.separator();

            match &self.selected_file {
                Some(file) => {
                    ui.label("Geselecteerd bestand:");
                    ui.label(file);
                }
                None => {
                    ui.label("Nog geen bestand geselecteerd.");
                }
            }
        });
    }
}