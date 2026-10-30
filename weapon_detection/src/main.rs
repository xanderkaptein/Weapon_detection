
mod video;

use eframe::egui;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use video::{VideoFrame, VideoReader};

const VIDEO_FPS: u64 = 30;
const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / VIDEO_FPS);
const MAX_IMAGE_HEIGHT: f32 = 500.0;
const END_OF_VIDEO_MESSAGE: &str = "Einde van video bereikt.";

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Media Viewer",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(MediaApp::default()))),
    )
}

struct MediaApp {
    // Gegevens van het momenteel geopende bestand.
    file: Option<PathBuf>,
    file_size: Option<u64>,
    image_dimensions: Option<[usize; 2]>,

    // Afbeelding en videobeeld worden als GPU-textures weergegeven.
    image: Option<egui::TextureHandle>,
    video: Option<VideoReader>,
    video_frame: Option<egui::TextureHandle>,

    // Status van de videospeler en eventuele foutmelding.
    error: Option<String>,
    playing: bool,
    video_finished: bool,
    last_frame: Instant,
}

impl Default for MediaApp {
    fn default() -> Self {
        Self {
            file: None,
            file_size: None,
            image_dimensions: None,
            image: None,
            video: None,
            video_frame: None,
            error: None,
            playing: false,
            video_finished: false,
            last_frame: Instant::now(),
        }
    }
}

impl MediaApp {
    /// Verwijdert de huidige media en reset de bijbehorende status.
    fn clear_media(&mut self) {
        self.file = None;
        self.file_size = None;
        self.image_dimensions = None;
        self.image = None;
        self.video = None;
        self.video_frame = None;
        self.playing = false;
        self.video_finished = false;
        self.error = None;
    }

    /// Leest de basisinformatie van een bestand in.
    fn set_file_info(&mut self, path: &Path) {
        self.file = Some(path.to_path_buf());
        self.file_size = std::fs::metadata(path)
            .ok()
            .map(|metadata| metadata.len());
    }

    /// Opent een afbeelding en zet deze om naar een egui-texture.
    fn open_image(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.clear_media();

        match image::open(&path) {
            Ok(img) => {
                let dimensions = [img.width() as usize, img.height() as usize];
                let rgba_image = img.to_rgba8();

                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    dimensions,
                    rgba_image.as_raw(),
                );

                self.image = Some(ctx.load_texture(
                    "image",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));

                self.image_dimensions = Some(dimensions);
                self.set_file_info(&path);
            }
            Err(error) => {
                self.error = Some(format!("Afbeelding openen mislukt: {error}"));
            }
        }
    }

    /// Opent een video en toont direct het eerste beschikbare frame.
    fn open_video(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.clear_media();

        match VideoReader::open(&path) {
            Ok(reader) => {
                self.video = Some(reader);
                self.set_file_info(&path);
                self.last_frame = Instant::now();
                self.read_next_frame(ctx);
            }
            Err(error) => {
                self.error = Some(format!("Video openen mislukt: {error}"));
            }
        }
    }

    /// Zet een videoframe om naar RGBA en werkt de texture bij.
    fn display_video_frame(&mut self, ctx: &egui::Context, frame: VideoFrame) {
        let mut rgba_pixels = Vec::with_capacity(frame.width * frame.height * 4);

        // Media Foundation levert hier BGRA-pixels; egui verwacht RGBA.
        for pixel in frame.data.chunks_exact(4) {
            rgba_pixels.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
        }

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [frame.width, frame.height],
            &rgba_pixels,
        );

        // Hergebruik de texture tijdens het afspelen om onnodige allocaties
        // bij ieder frame te voorkomen.
        if let Some(texture) = &mut self.video_frame {
            texture.set(color_image, egui::TextureOptions::LINEAR);
        } else {
            self.video_frame = Some(ctx.load_texture(
                "video_frame",
                color_image,
                egui::TextureOptions::LINEAR,
            ));
        }
    }

    /// Leest het volgende frame en verwerkt het einde van de video.
    fn read_next_frame(&mut self, ctx: &egui::Context) -> bool {
        let result = match self.video.as_ref() {
            Some(video) => video.read_frame(),
            None => return false,
        };

        match result {
            Ok(frame) => {
                self.display_video_frame(ctx, frame);
                self.error = None;
                self.video_finished = false;
                true
            }
            Err(error) => {
                self.playing = false;

                if error == END_OF_VIDEO_MESSAGE {
                    self.video_finished = true;
                    self.error = None;
                } else {
                    self.error = Some(error);
                }

                false
            }
        }
    }

    /// Opent de video opnieuw, zodat deze vanaf het begin kan worden afgespeeld.
    fn restart_video(&mut self, ctx: &egui::Context) {
        let Some(path) = self.file.clone() else {
            return;
        };

        self.playing = false;
        self.video_finished = false;
        self.error = None;

        match VideoReader::open(&path) {
            Ok(reader) => {
                self.video = Some(reader);
                self.last_frame = Instant::now();
                self.read_next_frame(ctx);
            }
            Err(error) => {
                self.video = None;
                self.video_frame = None;
                self.error = Some(format!("Video opnieuw openen mislukt: {error}"));
            }
        }
    }

    /// Toont de naam, extensie en bestandsgrootte van de huidige media.
    fn show_file_info(&self, ui: &mut egui::Ui) {
        let Some(path) = &self.file else {
            return;
        };

        ui.group(|ui| {
            ui.heading("Bestandsinformatie");

            ui.label(format!(
                "Naam: {}",
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            ));

            ui.label(format!("Type: {}", file_type(path)));

            if let Some([width, height]) = self.image_dimensions {
                ui.label(format!("Afmetingen: {width} × {height} pixels"));
            } else if let Some(frame) = &self.video_frame {
                let [width, height] = frame.size();
                ui.label(format!("Videobeeld: {width} × {height} pixels"));
            }

            if let Some(bytes) = self.file_size {
                ui.label(format!("Bestandsgrootte: {}", format_file_size(bytes)));
            }
        });
    }

    /// Toont de geopende afbeelding met behoud van de beeldverhouding.
    fn show_image(&self, ui: &mut egui::Ui) {
        let Some(texture) = &self.image else {
            return;
        };

        ui.heading("Afbeelding");

        let original_size = texture.size_vec2();
        let available_width = ui.available_width().max(1.0);

        if original_size.x > 0.0 && original_size.y > 0.0 {
            let scale = (available_width / original_size.x)
                .min(MAX_IMAGE_HEIGHT / original_size.y)
                .min(1.0);

            ui.add(
                egui::Image::new(texture)
                    .fit_to_exact_size(original_size * scale),
            );
        }
    }

    /// Toont de videobediening en de huidige afspeelstatus.
    fn show_video_controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if self.video.is_none() {
            return;
        }

        ui.heading("Videospeler");

        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.playing && !self.video_finished,
                    egui::Button::new("▶ Afspelen"),
                )
                .clicked()
            {
                self.playing = true;
                self.last_frame = Instant::now();
                self.error = None;
            }

            if ui
                .add_enabled(self.playing, egui::Button::new("⏸ Pauzeren"))
                .clicked()
            {
                self.playing = false;
            }

            if ui.button("⏮ Terug naar begin").clicked() {
                self.restart_video(ctx);
            }
        });

        let status = if self.playing {
            "Video wordt afgespeeld"
        } else if self.video_finished {
            "Video afgelopen"
        } else {
            "Gepauzeerd"
        };

        ui.label(format!("Status: {status}"));

        if self.video_finished {
            ui.label("Ga terug naar het begin om de video opnieuw af te spelen.");
        }
    }
}

/// Bepaalt het bestandstype aan de hand van de extensie.
fn file_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("png" | "jpg" | "jpeg") => "Afbeelding",
        Some("mp4" | "avi" | "mov" | "mkv") => "Video",
        _ => "Onbekend bestandstype",
    }
}

/// Zet de bestandsgrootte om naar een leesbare eenheid.
fn format_file_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let size = bytes as f64;

    if size >= GB {
        format!("{:.2} GB", size / GB)
    } else if size >= MB {
        format!("{:.2} MB", size / MB)
    } else if size >= KB {
        format!("{:.2} KB", size / KB)
    } else {
        format!("{bytes} bytes")
    }
}

impl eframe::App for MediaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Media Viewer");
            ui.label("Afbeeldingen en video's bekijken");

            ui.separator();
            ui.heading("Bestand selecteren");

            ui.horizontal_wrapped(|ui| {
                if ui.button("Afbeelding openen").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Afbeeldingen", &["png", "jpg", "jpeg"])
                        .pick_file()
                    {
                        self.open_image(ctx, path);
                    }
                }

                if ui.button("Video openen").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Video's", &["mp4", "avi", "mov", "mkv"])
                        .pick_file()
                    {
                        self.open_video(ctx, path);
                    }
                }
            });

            ui.add_space(8.0);
            self.show_file_info(ui);

            ui.separator();
            self.show_image(ui);
            self.show_video_controls(ui, ctx);

            if let Some(texture) = &self.video_frame {
                ui.heading("Videobeeld");

                egui::ScrollArea::both()
                    .max_height(550.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Image::new(texture)
                                .max_width(900.0)
                                .max_height(500.0),
                        );
                    });
            }

            if self.file.is_none() && self.error.is_none() {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.label("Nog geen bestand geselecteerd.");
                    ui.label("Open een afbeelding of video om te beginnen.");
                });
            }

            if let Some(error) = &self.error {
                ui.separator();
                ui.colored_label(
                    egui::Color32::RED,
                    format!("Foutmelding: {error}"),
                );
            }
        });

        // Vraag alleen nieuwe frames op terwijl de video wordt afgespeeld.
        if self.playing {
            if self.last_frame.elapsed() >= FRAME_DURATION {
                self.last_frame = Instant::now();
                self.read_next_frame(ctx);
            }

            ctx.request_repaint_after(Duration::from_millis(5));
        }
    }
}
