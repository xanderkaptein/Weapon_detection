mod video;

use eframe::egui;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use video::VideoReader;

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
            last_frame: Instant::now(),
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
            ui.separator();

            // -------------------------------
            // Afbeelding openen
            // -------------------------------

            if ui.button("Afbeelding openen").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "Afbeeldingen",
                        &["png", "jpg", "jpeg"],
                    )
                    .pick_file()
                {
                    self.file = Some(path.clone());

                    self.video = None;
                    self.video_frame = None;
                    self.playing = false;
                    self.error = None;

                    match image::open(&path) {
                        Ok(img) => {
                            let img = img.to_rgba8();

                            let size = [
                                img.width() as usize,
                                img.height() as usize,
                            ];

                            let color =
                                egui::ColorImage::from_rgba_unmultiplied(
                                    size,
                                    img.as_raw(),
                                );

                            self.image = Some(
                                ctx.load_texture(
                                    "image",
                                    color,
                                    Default::default(),
                                ),
                            );
                        }

                        Err(e) => {
                            self.image = None;
                            self.error =
                                Some(format!(
                                    "Afbeelding openen mislukt: {e}"
                                ));
                        }
                    }
                }
            }

            // -------------------------------
            // Video openen
            // -------------------------------

            if ui.button("Video openen").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "Video",
                        &["mp4", "avi", "mov", "mkv"],
                    )
                    .pick_file()
                {
                    self.file = Some(path.clone());

                    self.image = None;
                    self.video_frame = None;
                    self.playing = false;
                    self.error = None;

                    match VideoReader::open(&path) {
                        Ok(reader) => {
                            self.video = Some(reader);
                        }

                        Err(e) => {
                            self.video = None;
                            self.error = Some(e);
                        }
                    }
                }
            }

            ui.separator();

            // -------------------------------
            // Geselecteerd bestand
            // -------------------------------

            if let Some(path) = &self.file {
                ui.label("Geselecteerd bestand:");
                ui.label(path.display().to_string());
            } else {
                ui.label("Nog geen bestand geselecteerd.");
            }

            // -------------------------------
            // Afbeelding
            // -------------------------------

            if let Some(texture) = &self.image {
                ui.separator();
                ui.label("Afbeelding:");

                ui.add(
                    egui::Image::new(texture)
                        .max_width(600.0)
                        .max_height(400.0),
                );
            }

            // -------------------------------
            // Video
            // -------------------------------

            if self.video.is_some() {
                ui.separator();
                ui.label("Video geopend.");

                if ui.button("Eerste frame uitlezen").clicked() {
                    if let Some(video) = &self.video {
                        match video.read_frame() {
                            Ok(frame) => {
                                let mut pixels =
                                    Vec::with_capacity(
                                        frame.width
                                            * frame.height
                                            * 4,
                                    );

                                for p in frame.data.chunks_exact(4) {
                                    pixels.extend([
                                        p[2], // R
                                        p[1], // G
                                        p[0], // B
                                        255,  // A
                                    ]);
                                }

                                let color =
                                    egui::ColorImage::from_rgba_unmultiplied(
                                        [frame.width, frame.height],
                                        &pixels,
                                    );

                                self.video_frame = Some(
                                    ctx.load_texture(
                                        "video_frame",
                                        color,
                                        Default::default(),
                                    ),
                                );

                                self.error = None;
                            }

                            Err(e) => {
                                self.error = Some(e);
                            }
                        }
                    }
                }

                ui.horizontal(|ui| {
                    if ui.button("▶ Video afspelen").clicked() {
                        self.playing = true;
                        self.last_frame = Instant::now();
                        self.error = None;
                    }

                    if ui.button("⏸ Pauzeren").clicked() {
                        self.playing = false;
                    }
                });
            }

            // -------------------------------
            // Video afspelen
            // -------------------------------

            if self.playing
                && self.last_frame.elapsed()
                    >= Duration::from_millis(33)
            {
                if let Some(video) = &self.video {
                    match video.read_frame() {
                        Ok(frame) => {
                            let mut pixels =
                                Vec::with_capacity(
                                    frame.width
                                        * frame.height
                                        * 4,
                                );

                            for p in frame.data.chunks_exact(4) {
                                pixels.extend([
                                    p[2], // R
                                    p[1], // G
                                    p[0], // B
                                    255,  // A
                                ]);
                            }

                            let color =
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [frame.width, frame.height],
                                    &pixels,
                                );

                            if let Some(texture) =
                                &mut self.video_frame
                            {
                                texture.set(
                                    color,
                                    Default::default(),
                                );
                            } else {
                                self.video_frame = Some(
                                    ctx.load_texture(
                                        "video_frame",
                                        color,
                                        Default::default(),
                                    ),
                                );
                            }

                            self.last_frame = Instant::now();
                        }

                        Err(_) => {
                            self.playing = false;
                        }
                    }
                }
            }

            // -------------------------------
            // Videoframe tonen
            // -------------------------------

            if let Some(texture) = &self.video_frame {
                ui.separator();
                ui.label("Videoframe:");

                ui.add(
                    egui::Image::new(texture)
                        .max_width(800.0)
                        .max_height(600.0),
                );
            }

            // -------------------------------
            // Foutmelding
            // -------------------------------

            if let Some(error) = &self.error {
                ui.separator();

                ui.colored_label(
                    egui::Color32::RED,
                    format!("Fout: {error}"),
                );
            }
        });

        // Zorg dat de GUI tijdens het afspelen
        // regelmatig opnieuw wordt getekend.
        if self.playing {
            ctx.request_repaint_after(
                Duration::from_millis(10),
            );
        }
    }
}