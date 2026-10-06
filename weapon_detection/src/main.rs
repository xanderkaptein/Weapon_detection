
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
    image_texture: Option<egui::TextureHandle>,
    image_error: Option<String>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Weapon Detection");

            ui.separator();

            // Afbeelding openen
            if ui.button("Afbeelding openen").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Afbeeldingen", &["png", "jpg", "jpeg"])
                    .pick_file()
                {
                    self.selected_file = Some(file.display().to_string());
                    self.image_error = None;
                    self.image_texture = None;

                    match image::open(&file) {
                        Ok(image) => {
                            let image = image.to_rgba8();

                            let size = [
                                image.width() as usize,
                                image.height() as usize,
                            ];

                            let color_image =
                                egui::ColorImage::from_rgba_unmultiplied(
                                    size,
                                    image.as_raw(),
                                );

                            self.image_texture = Some(
                                ctx.load_texture(
                                    "selected_image",
                                    color_image,
                                    egui::TextureOptions::default(),
                                ),
                            );
                        }

                        Err(error) => {
                            self.image_error = Some(error.to_string());
                        }
                    }
                }
            }

            // Video selecteren
            if ui.button("Video openen").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Videobestanden", &["mp4", "avi", "mov"])
                    .pick_file()
                {
                    self.selected_file = Some(file.display().to_string());

                    // Een vorige afbeelding niet meer tonen
                    self.image_texture = None;
                    self.image_error = None;
                }
            }

            ui.separator();

            // Geselecteerd bestand weergeven
            match &self.selected_file {
                Some(file) => {
                    ui.label("Geselecteerd bestand:");
                    ui.label(file);
                }

                None => {
                    ui.label("Nog geen bestand geselecteerd.");
                }
            }

            // Foutmelding bij het laden van een afbeelding
            if let Some(error) = &self.image_error {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("Fout bij laden van afbeelding: {}", error),
                );
            }

            // Afbeelding tonen
            if let Some(texture) = &self.image_texture {
                ui.add(
                    egui::Image::new(texture)
                        .max_width(600.0)
                        .max_height(400.0),
                );
            }
        });
    }
}