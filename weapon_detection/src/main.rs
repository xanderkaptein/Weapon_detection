
mod video;

use eframe::egui;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use video::VideoReader;

const VIDEO_FPS: f64 = 30.0;
const FRAME_DURATION: Duration =
    Duration::from_micros(1_000_000 / VIDEO_FPS as u64);

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Weapon Detection",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct MyApp {
    file: Option<PathBuf>,
    image: Option<egui::TextureHandle>,
    video: Option<VideoReader>,
    video_frame: Option<egui::TextureHandle>,
    error: Option<String>,
    playing: bool,
    video_finished: bool,
    last_frame: Instant,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            file: None,
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

impl MyApp {
    fn display_video_frame(
        &mut self,
        ctx: &egui::Context,
        frame: video::VideoFrame,
    ) {
        let mut pixels =
            Vec::with_capacity(frame.width * frame.height * 4);

        for pixel in frame.data.chunks_exact(4) {
            pixels.extend([
                pixel[2],
                pixel[1],
                pixel[0],
                255,
            ]);
        }

        let color_image =
            egui::ColorImage::from_rgba_unmultiplied(
                [frame.width, frame.height],
                &pixels,
            );

        if let Some(texture) = &mut self.video_frame {
            texture.set(
                color_image,
                egui::TextureOptions::LINEAR,
            );
        } else {
            self.video_frame = Some(
                ctx.load_texture(
                    "video_frame",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ),
            );
        }
    }

    fn read_next_frame(
        &mut self,
        ctx: &egui::Context,
    ) -> bool {
        let Some(video) = &self.video else {
            return false;
        };

        match video.read_frame() {
            Ok(frame) => {
                self.display_video_frame(ctx, frame);
                self.error = None;
                self.video_finished = false;
                true
            }
            Err(error) => {
                self.playing = false;

                if error == "Einde van video bereikt." {
                    self.error = None;
                    self.video_finished = true;
                } else {
                    self.error = Some(error);
                }

                false
            }
        }
    }

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
                self.read_next_frame(ctx);
                self.last_frame = Instant::now();
            }
            Err(error) => {
                self.video = None;
                self.error = Some(error);
            }
        }
    }

    fn open_image(
        &mut self,
        ctx: &egui::Context,
        path: PathBuf,
    ) {
        self.file = Some(path.clone());
        self.image = None;
        self.video = None;
        self.video_frame = None;
        self.playing = false;
        self.video_finished = false;
        self.error = None;

        match image::open(&path) {
            Ok(img) => {
                let img = img.to_rgba8();

                let size = [
                    img.width() as usize,
                    img.height() as usize,
                ];

                let color_image =
                    egui::ColorImage::from_rgba_unmultiplied(
                        size,
                        img.as_raw(),
                    );

                self.image = Some(
                    ctx.load_texture(
                        "image",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    ),
                );
            }
            Err(error) => {
                self.error = Some(format!(
                    "Afbeelding openen mislukt: {error}"
                ));
            }
        }
    }

    fn open_video(
        &mut self,
        ctx: &egui::Context,
        path: PathBuf,
    ) {
        self.file = Some(path.clone());
        self.image = None;
        self.video = None;
        self.video_frame = None;
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
                self.error = Some(error);
            }
        }
    }
}

impl eframe::App for MyApp {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Weapon Detection");
            ui.label("Afbeeldingen en video's bekijken");

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.heading("Bestand selecteren");

            ui.horizontal_wrapped(|ui| {
                if ui.button("Afbeelding openen").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            "Afbeeldingen",
                            &["png", "jpg", "jpeg"],
                        )
                        .pick_file()
                    {
                        self.open_image(ctx, path);
                    }
                }

                if ui.button("Video openen").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            "Video's",
                            &["mp4", "avi", "mov", "mkv"],
                        )
                        .pick_file()
                    {
                        self.open_video(ctx, path);
                    }
                }
            });

            ui.add_space(8.0);

            if let Some(path) = &self.file {
                ui.group(|ui| {
                    ui.label("Geselecteerd bestand");
                    ui.label(path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy());

                    ui.label(format!(
                        "Type: {}",
                        if self.video.is_some() {
                            "Video"
                        } else if self.image.is_some() {
                            "Afbeelding"
                        } else {
                            "Niet geladen"
                        }
                    ));
                });
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            if let Some(texture) = &self.image {
                ui.heading("Afbeelding");
                ui.add_space(4.0);

                egui::ScrollArea::both()
                    .max_height(550.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Image::new(texture)
                                .max_width(800.0)
                                .max_height(500.0),
                        );
                    });
            }

            if self.video.is_some() {
                ui.heading("Videospeler");
                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            !self.playing && !self.video_finished,
                            egui::Button::new("▶ Afspelen"),
                        )
                        .clicked()
                    {
                        self.playing = true;
                        self.error = None;
                        self.last_frame = Instant::now();
                    }

                    if ui
                        .add_enabled(
                            self.playing,
                            egui::Button::new("⏸ Pauzeren"),
                        )
                        .clicked()
                    {
                        self.playing = false;
                    }

                    if ui.button("⏮ Terug naar begin").clicked() {
                        self.restart_video(ctx);
                    }
                });

                ui.add_space(4.0);

                ui.label(if self.playing {
                    "Status: Video wordt afgespeeld"
                } else if self.video_finished {
                    "Status: Video afgelopen"
                } else {
                    "Status: Gepauzeerd"
                });

                if self.video_finished {
                    ui.label(
                        "Ga terug naar het begin om de video opnieuw af te spelen."
                    );
                }
            }

            if let Some(texture) = &self.video_frame {
                ui.add_space(8.0);
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

            if self.image.is_none()
                && self.video.is_none()
                && self.video_frame.is_none()
                && self.error.is_none()
            {
                ui.add_space(24.0);

                ui.vertical_centered(|ui| {
                    ui.label("Nog geen bestand geselecteerd.");
                    ui.label(
                        "Open een afbeelding of video om te beginnen."
                    );
                });
            }

            if let Some(error) = &self.error {
                ui.add_space(8.0);
                ui.separator();
                ui.colored_label(
                    egui::Color32::RED,
                    format!("Foutmelding: {error}"),
                );
            }
        });

        if self.playing {
            if self.last_frame.elapsed() >= FRAME_DURATION {
                self.last_frame = Instant::now();
                self.read_next_frame(ctx);
            }

            ctx.request_repaint_after(Duration::from_millis(5));
        }
    }
}
