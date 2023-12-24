use crate::{PrintData, PrintQueue};
use rocket::State;
use serde::{Serialize, Deserialize};
use rocket::serde::json::Json;
use rocket::fs::TempFile;
use rocket::form::Form;
use rocket::tokio::io::AsyncReadExt;
use std::io::Cursor;
use std::sync::Arc;
use image::GrayImage;
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
}

#[derive(FromForm)]
pub struct PrintJobImageRequest<'a>{
    pub image: TempFile<'a>,
    pub quantity: Option<u32>,
    pub rotate: Option<bool>,
}

#[derive(FromForm)]
pub struct PrintJobTextRequest{
    pub text: String,
    pub quantity: Option<u32>,
    pub invert: Option<bool>,
    pub length: u32,
    pub rotate: Option<bool>,
}

/// Get the current print queue
///
/// # Arguments
/// * `status` - Optional filter for the print queue ('pending', 'printing', 'complete', 'failed')
#[get("/queue?<status>")]
pub fn get_print_queue(status: Option<String>){

}

#[post("/print/text", data="<data>")]
pub async fn add_text_to_queue(data: Form<PrintJobTextRequest>, queue: &State<Arc<PrintQueue>>) -> Json<PrintJobResponse> {
    let data = data.into_inner();

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

    let image = match text_to_image(data.text, pixel_length, data.rotate.unwrap_or(false), data.invert.unwrap_or(false), FontVariants::Arial){
        Ok(image) => image,
        Err(_) => return Json(PrintJobResponse{ id: None, error: Some(ApiError::InvalidRequest) })
    };

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    println!("Adding job to queue.");
    let id = queue.add_job(now, PrintData::Image(image.clone()), data.quantity.unwrap_or(1));

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
        let reader = match image::io::Reader::new(Cursor::new(buf.as_slice())).with_guessed_format(){
            Ok(reader) => reader,
            Err(_) => return Err(Json(PrintJobResponse{ id: None, error: Some(ApiError::InvalidImageFile) }))
        };
        println!("Decoding image file.");
        let mut image = match reader.decode(){
            Ok(image) => image,
            Err(_) => return Err(Json(PrintJobResponse{ id: None, error: Some(ApiError::InvalidImageFile) }))
        };
        println!("Rotating image if necessarry");
        if let Some(rotate) = data.rotate{
            if rotate{
                image = image.rotate90();
            }
        }

        let nheight : u32 = 704; //704 px ~= 62mm

        let nwidth = image.height();

        println!("Resizing image.");

        image = image.resize(nwidth, nheight, image::imageops::FilterType::Triangle);

        println!("Converting to grayscale.");
        image = image.grayscale();

        println!("New image dimensions are {} x {}", image.width(), image.height());

        let image = image.into_luma8();

        Ok(image)
    }).await.unwrap();

    let image = match processing_res{
        Ok(image) => image,
        Err(res) => return res
    };

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    println!("Adding job to queue.");
    let id = queue.add_job(now, PrintData::Image(image.clone()), quantity);

    println!("Added new job {} to queue", id);

    Json(PrintJobResponse{
        id: Some(id),
        error: None
    })
}