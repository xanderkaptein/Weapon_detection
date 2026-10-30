
use ab_glyph::{FontArc, PxScale};
use image::{imageops::FilterType, Rgb, RgbImage};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use ort::{session::Session, value::Tensor};
use std::{error::Error, fs};

const MODEL_PATH: &str = "models/yolo26n.onnx";
const IMAGE_PATH: &str = "testdata/test.png";
const OUTPUT_PATH: &str = "testdata/test_result.png";

const SIZE: u32 = 640;
const CANDIDATES: usize = 8400;
const CLASSES: usize = 80;
const CONFIDENCE: f32 = 0.35;
const NMS_THRESHOLD: f32 = 0.45;

const LABELS: [&str; 80] = [
    "person", "bicycle", "car", "motorcycle", "airplane",
    "bus", "train", "truck", "boat", "traffic light",
    "fire hydrant", "stop sign", "parking meter", "bench",
    "bird", "cat", "dog", "horse", "sheep", "cow",
    "elephant", "bear", "zebra", "giraffe", "backpack",
    "umbrella", "handbag", "tie", "suitcase", "frisbee",
    "skis", "snowboard", "sports ball", "kite",
    "baseball bat", "baseball glove", "skateboard",
    "surfboard", "tennis racket", "bottle", "wine glass",
    "cup", "fork", "knife", "spoon", "bowl", "banana",
    "apple", "sandwich", "orange", "broccoli", "carrot",
    "hot dog", "pizza", "donut", "cake", "chair", "couch",
    "potted plant", "bed", "dining table", "toilet", "tv",
    "laptop", "mouse", "remote", "keyboard", "cell phone",
    "microwave", "oven", "toaster", "sink", "refrigerator",
    "book", "clock", "vase", "scissors", "teddy bear",
    "hair drier", "toothbrush",
];

#[derive(Clone)]
struct Detection {
    class_id: usize,
    score: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

fn iou(a: &Detection, b: &Detection) -> f32 {
    let x1 = a.x1.max(b.x1);
    let y1 = a.y1.max(b.y1);
    let x2 = a.x2.min(b.x2);
    let y2 = a.y2.min(b.y2);

    let intersection = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    let area_a = (a.x2 - a.x1).max(0.0)
        * (a.y2 - a.y1).max(0.0);
    let area_b = (b.x2 - b.x1).max(0.0)
        * (b.y2 - b.y1).max(0.0);

    intersection / (area_a + area_b - intersection + 1e-6)
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Afbeelding laden: {IMAGE_PATH}");

    let original = image::open(IMAGE_PATH)?.to_rgb8();
    let (original_width, original_height) = original.dimensions();

    let resized = image::imageops::resize(
        &original,
        SIZE,
        SIZE,
        FilterType::Triangle,
    );

    let mut input_data =
        Vec::with_capacity(3 * SIZE as usize * SIZE as usize);

    for channel in 0..3 {
        for y in 0..SIZE {
            for x in 0..SIZE {
                input_data.push(
                    resized.get_pixel(x, y)[channel] as f32 / 255.0,
                );
            }
        }
    }

    println!("ONNX-model laden...");

    let environment = ort::init()
        .with_name("standalone-object-demo")
        .build()?;

    let mut session = Session::builder(&environment)?
        .commit_from_file(MODEL_PATH)?;

    let tensor = Tensor::from_array((
        [1usize, 3, SIZE as usize, SIZE as usize],
        input_data,
    ))?;

    println!("Inferentie uitvoeren...");

    let outputs = session.run(ort::inputs! {
        "images" => tensor
    })?;

    let (_, values) =
        outputs["output0"].try_extract_tensor::<f32>()?;

    if values.len() != (4 + CLASSES) * CANDIDATES {
        return Err(format!(
            "Onverwacht aantal uitvoerwaarden: {}",
            values.len()
        ).into());
    }

    println!("Detecties verwerken...");

    let mut detections = Vec::new();

    for i in 0..CANDIDATES {
        let mut best_class = 0;
        let mut best_score = 0.0_f32;

        for class_id in 0..CLASSES {
            let score = values[(4 + class_id) * CANDIDATES + i];

            if score > best_score {
                best_score = score;
                best_class = class_id;
            }
        }

        if !best_score.is_finite() || best_score < CONFIDENCE {
            continue;
        }

        let cx = values[i];
        let cy = values[CANDIDATES + i];
        let w = values[2 * CANDIDATES + i];
        let h = values[3 * CANDIDATES + i];

        if !cx.is_finite()
            || !cy.is_finite()
            || !w.is_finite()
            || !h.is_finite()
            || w <= 0.0
            || h <= 0.0
        {
            continue;
        }

        detections.push(Detection {
            class_id: best_class,
            score: best_score,
            x1: (cx - w / 2.0).max(0.0),
            y1: (cy - h / 2.0).max(0.0),
            x2: (cx + w / 2.0).min(SIZE as f32),
            y2: (cy + h / 2.0).min(SIZE as f32),
        });
    }

    detections.sort_by(|a, b| {
        b.score.total_cmp(&a.score)
    });

    // Non-maximum suppression: dubbele kaders beperken.
    let mut filtered: Vec<Detection> = Vec::new();

    for detection in detections {
        let overlaps = filtered.iter().any(|kept| {
            kept.class_id == detection.class_id
                && iou(kept, &detection) > NMS_THRESHOLD
        });

        if !overlaps {
            filtered.push(detection);
        }

        if filtered.len() >= 100 {
            break;
        }
    }

    println!("Aantal detecties: {}", filtered.len());

    let mut result: RgbImage = original.clone();

    let font = fs::read(r"C:\Windows\Fonts\arial.ttf")
        .ok()
        .and_then(|bytes| FontArc::try_from_vec(bytes).ok());

    let scale_x = original_width as f32 / SIZE as f32;
    let scale_y = original_height as f32 / SIZE as f32;

    for detection in &filtered {
        let x1 = (detection.x1 * scale_x)
            .clamp(0.0, original_width as f32 - 1.0);
        let y1 = (detection.y1 * scale_y)
            .clamp(0.0, original_height as f32 - 1.0);
        let x2 = (detection.x2 * scale_x)
            .clamp(0.0, original_width as f32 - 1.0);
        let y2 = (detection.y2 * scale_y)
            .clamp(0.0, original_height as f32 - 1.0);

        let width = (x2 - x1).max(1.0) as u32;
        let height = (y2 - y1).max(1.0) as u32;

        draw_hollow_rect_mut(
            &mut result,
            Rect::at(x1 as i32, y1 as i32)
                .of_size(width, height),
            Rgb([0, 255, 0]),
        );

        if let Some(font) = &font {
            let label = format!(
                "{} {:.0}%",
                LABELS[detection.class_id],
                detection.score * 100.0
            );

            draw_text_mut(
                &mut result,
                Rgb([255, 255, 0]),
                x1 as i32,
                (y1 - 20.0).max(0.0) as i32,
                PxScale::from(18.0),
                font,
                &label,
            );
        }
    }

    result.save(OUTPUT_PATH)?;

    println!("Resultaat opgeslagen: {OUTPUT_PATH}");
    println!("Klaar!");

    // Open de afbeelding met de standaard Windows-app.
    std::process::Command::new("explorer")
        .arg(OUTPUT_PATH)
        .spawn()?;

    Ok(())
}
