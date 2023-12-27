use crate::{PrintJob, PrintQueue};
use rocket::State;
use serde::{Serialize, Deserialize};
use rocket::serde::json::Json;
use rocket::fs::TempFile;
use rocket::form::Form;
use rocket::tokio::io::AsyncReadExt;
use std::io::Cursor;
use std::sync::Arc;
use image::{GrayImage, ImageFormat};
use crate::printer::{FontVariants, text_to_image};

#[derive(Serialize, Deserialize)]
pub struct PrintJobResponse{
    pub id: Option<u32>,
    pub error: Option<ApiError>,
}

#[derive(Serialize, Deserialize)]
pub enum ApiError{
    PrinterError,
    InvalidImageFile,
    InvalidRequest,
    LengthTooLong,
    QuantityTooHigh,
    QueueTooFull,
    UnknownFont,
}

#[derive(FromForm)]
pub struct PrintJobImageRequest<'a>{
    pub image: TempFile<'a>,
    pub quantity: Option<u32>,
    pub rotate: Option<bool>,
    pub dither: Option<bool>,
}

#[derive(FromForm)]
pub struct PrintJobTextRequest{
    pub text: String,
    pub quantity: Option<u32>,
    pub invert: Option<bool>,
    pub length: u32,
    pub rotate: Option<bool>,
    pub font: Option<String>
}

#[derive(Serialize)]
pub struct ApiQueueList{
    pub jobs_todo: Vec<PrintJob>,
    pub jobs_finished_or_failed: Vec<PrintJob>,
}

/// Get the current print queue as JSON
#[get("/queue")]
pub fn get_print_queue(print_queue: &State<Arc<PrintQueue>>) -> Json<ApiQueueList>{
    Json(ApiQueueList{
        jobs_todo: print_queue.jobs_todo.read().unwrap().clone().iter().map(|job| job.clone()).collect::<Vec<PrintJob>>(),
        jobs_finished_or_failed: print_queue.jobs_other.read().unwrap().clone().iter().map(|job| job.clone()).collect::<Vec<PrintJob>>(),
    })
}

#[get("/queue/<id>")]
pub fn get_print_job(id: u32, print_queue: &State<Arc<PrintQueue>>) -> Option<Json<PrintJob>>{
    let jobs_todo = print_queue.jobs_todo.read().unwrap();
    let jobs_other = print_queue.jobs_other.read().unwrap();

    for job in jobs_todo.iter(){
        if job.id == id{
            return Some(Json(job.clone()))
        }
    }

    for job in jobs_other.iter(){
        if job.id == id{
            return Some(Json(job.clone()))
        }
    }

    None
}

#[post("/print/text", data="<data>")]
pub async fn add_text_to_queue(data: Form<PrintJobTextRequest>, queue: &State<Arc<PrintQueue>>) -> Json<PrintJobResponse> {
    let data = data.into_inner();

    println!("Adding text to queue: {}", data.text.clone());

    let font = match data.font{
        Some(font) => {
            match FontVariants::from_str(&font) {
                Some(font) => font,
                None => return Json(PrintJobResponse{ id: None, error: Some(ApiError::UnknownFont) })
            }
        },
        None => {
            FontVariants::Arial
        }
    };

    if data.quantity.unwrap_or(1) > 10{
        return Json(PrintJobResponse{ id: None, error: Some(ApiError::QuantityTooHigh) })
    }

    if queue.jobs_todo.read().unwrap().len() > 20{
        return Json(PrintJobResponse{ id: None, error: Some(ApiError::QueueTooFull) })
    }

    if data.length > 500{
        return Json(PrintJobResponse{ id: None, error: Some(ApiError::LengthTooLong) })
    }

    let pixel_length = (11.3548387097 * data.length as f32) as u32;

    let image = match text_to_image(data.text, pixel_length, data.rotate.unwrap_or(true), data.invert.unwrap_or(false), font){
        Ok(image) => image,
        Err(_) => return Json(PrintJobResponse{ id: None, error: Some(ApiError::InvalidRequest) })
    };

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    println!("Adding job to queue.");
    let id = queue.add_job(now, image.clone(), data.quantity.unwrap_or(1));

    println!("Added new job {} to queue", id);

    image.save("debug_text.png").unwrap();
    Json(PrintJobResponse{
        id: Some(id),
        error: None
    })
}

#[post("/print/image", data="<data>")]
pub async fn add_image_to_queue(data: Form<PrintJobImageRequest<'_>>, queue: &State<Arc<PrintQueue>>) -> Json<PrintJobResponse>{
    let mut buf: Vec<u8> = Vec::new();
    let data = data.into_inner();

    let quantity = match data.quantity.clone(){
        Some(quantity) => {
            if quantity > 10{
                return Json(PrintJobResponse{ id: None, error: Some(ApiError::QuantityTooHigh) })
            }else{
                quantity
            }
        },
        None => 1
    };

    if queue.jobs_todo.read().unwrap().len() > 20{
        return Json(PrintJobResponse{ id: None, error: Some(ApiError::QueueTooFull) })
    }

    println!("Reading image file into buffer.");
    match data.image.open().await.unwrap().read_to_end(&mut buf).await{
        Ok(file) => file,
        Err(_) => return Json(PrintJobResponse{ id: None, error: Some(ApiError::InvalidImageFile) })
    };

    let processing_res : Result<GrayImage, Json<PrintJobResponse>>= tokio::task::spawn_blocking(move || {
        println!("Parsing image file.");
        let mut reader = match image::io::Reader::new(Cursor::new(buf.as_slice())).with_guessed_format(){
            Ok(reader) => reader,
            Err(e) =>
                {
                    println!("{}", e);
                    return Err(Json(PrintJobResponse { id: None, error: Some(ApiError::InvalidImageFile) }))
                }
        };
        println!("Decoding image file.");
        let mut image = match reader.decode(){
            Ok(image) => image,
            Err(e) => {
                println!("Couldn't decode image: {}", e);
                return Err(Json(PrintJobResponse{ id: None, error: Some(ApiError::InvalidImageFile) }))
            }
        };
        println!("Rotating image if necessarry");
        if let Some(rotate) = data.rotate{
            if rotate{
                image = image.rotate90();
                println!("Rotated image.");
            }
        }

        println!("Old image dimensions are {} x {}", image.width(), image.height());
        let height_diff = 704.0 / image.height() as f32;
        println!("Height difference is {}", height_diff);
        let nheight : u32 = 704; //704 px ~= 62mm
        let nwidth = (image.width() as f32 *height_diff).round() as u32;

        println!("Resizing image: {} x {}", nwidth, nheight);


        image = image.resize_exact(nwidth, nheight, image::imageops::FilterType::Triangle);

        if data.dither.unwrap_or(true) {
            let mut image = image.into_luma8();
            image::imageops::dither(&mut image, &image::imageops::colorops::BiLevel);
            Ok(image)
        }else{
            image.grayscale();
            Ok(image.into_luma8())
        }
    }).await.unwrap();

    let image = match processing_res{
        Ok(image) => image,
        Err(res) => return res
    };

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    println!("Adding job to queue.");
    let id = queue.add_job(now, image.clone(), quantity);

    println!("Added new job {} to queue", id);

    Json(PrintJobResponse{
        id: Some(id),
        error: None
    })
}