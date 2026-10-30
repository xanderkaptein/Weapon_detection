use image::{DynamicImage, GenericImageView};
use ort::session::Session;
use ort::value::Tensor;

const MODEL_PATH: &str = "models/yolo26n.onnx";

const INPUT_WIDTH: usize = 640;
const INPUT_HEIGHT: usize = 640;

// COCO class 0 = person
const PERSON_CLASS: usize = 0;

// Alleen resultaten boven deze confidence gebruiken.
const CONFIDENCE_THRESHOLD: f32 = 0.40;

#[derive(Debug, Clone)]
pub struct Detection {
    pub class_id: usize,
    pub confidence: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

pub struct YoloModel {
    session: Session,
}

impl YoloModel {
    pub fn new() -> Result<Self, String> {
        println!("ONNX-model laden...");

        let environment = ort::init()
            .with_name("person-detection")
            .build()
            .map_err(|e| {
                format!("ONNX Runtime initialiseren mislukt: {e}")
            })?;

        let session = Session::builder(&environment)
            .map_err(|e| {
                format!("ONNX SessionBuilder aanmaken mislukt: {e}")
            })?
            .commit_from_file(MODEL_PATH)
            .map_err(|e| {
                format!("Model laden mislukt: {e}")
            })?;

        println!("ONNX-model geladen.");

        Ok(Self { session })
    }

    pub fn detect(&mut self, image: &DynamicImage) -> Result<Vec<Detection>, String> {
        let resized = image.resize_exact(
            INPUT_WIDTH as u32,
            INPUT_HEIGHT as u32,
            image::imageops::FilterType::Triangle,
        );

        let rgb = resized.to_rgb8();

        let mut input_data = vec![0.0f32; 3 * INPUT_WIDTH * INPUT_HEIGHT];

        let plane_size = INPUT_WIDTH * INPUT_HEIGHT;

        for y in 0..INPUT_HEIGHT {
            for x in 0..INPUT_WIDTH {
                let pixel = rgb.get_pixel(x as u32, y as u32);

                let index = y * INPUT_WIDTH + x;

                input_data[index] = pixel[0] as f32 / 255.0;
                input_data[plane_size + index] = pixel[1] as f32 / 255.0;
                input_data[2 * plane_size + index] = pixel[2] as f32 / 255.0;
            }
        }

        let input = Tensor::from_array(
            ([1usize, 3, INPUT_HEIGHT, INPUT_WIDTH], input_data)
        )
        .map_err(|e| format!("Tensor aanmaken mislukt: {e}"))?;

        println!("Inferentie uitvoeren...");

        let outputs = self
            .session
            .run(ort::inputs!["images" => input])
            .map_err(|e| format!("Inferentie mislukt: {e}"))?;

        println!("Inferentie geslaagd.");

        let output = outputs
            .get("output0")
            .ok_or("Output 'output0' niet gevonden.")?;

        let (_shape, output_data) = output
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Output uitlezen mislukt: {e}"))?;

        println!("Detecties verwerken...");

        let mut detections = Vec::new();

        // YOLO26n output:
        // [1, 84, 8400]
        //
        // De eerste 4 waarden zijn:
        // x, y, width, height
        //
        // De overige waarden bevatten de class scores.

        for candidate in 0..8400usize {
            let x = output_data[candidate];
            let y = output_data[8400 + candidate];
            let width = output_data[2 * 8400 + candidate];
            let height = output_data[3 * 8400 + candidate];

            let mut best_class = 0usize;
            let mut best_score = 0.0f32;

            for class_id in 0..80usize {
                let score = output_data[(4 + class_id) * 8400 + candidate];

                if score > best_score {
                    best_score = score;
                    best_class = class_id;
                }
            }

            if best_class != PERSON_CLASS {
                continue;
            }

            if best_score < CONFIDENCE_THRESHOLD {
                continue;
            }

            let x1 = (x - width / 2.0).max(0.0);
            let y1 = (y - height / 2.0).max(0.0);
            let x2 = (x + width / 2.0).min(INPUT_WIDTH as f32);
            let y2 = (y + height / 2.0).min(INPUT_HEIGHT as f32);

            detections.push(Detection {
                class_id: best_class,
                confidence: best_score,
                x1,
                y1,
                x2,
                y2,
            });
        }

        // Grootste confidence eerst.
        detections.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        println!("Aantal personen: {}", detections.len());

        Ok(detections)
    }
}