use std::collections::VecDeque;
use std::process;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, RwLock};
use image::GrayImage;
use rocket::tokio;
use serde::Serialize;

#[macro_use]
extern crate rocket;

pub mod api;
pub mod printer;

#[derive(Serialize)]
pub struct PrintQueue {
    pub jobs_todo: RwLock<VecDeque<PrintJob>>,
    pub jobs_other: RwLock<VecDeque<PrintJob>>,
    counter: AtomicU32,
}

impl PrintQueue {
    pub fn new() -> PrintQueue {
        PrintQueue {
            jobs_todo: Default::default(),
            counter: AtomicU32::new(0),
            jobs_other: Default::default(),
        }
    }
    pub fn add_job(&self, timestamp: u64, data: GrayImage, quantity: u32) -> u32 {
        let id = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let print_job = PrintJob {
            id,
            timestamp,
            quantity,
            data: Some(data),
            status: PrintJobStatus::Pending,
        };
        self.jobs_todo.write().unwrap().push_back(print_job);
        id
    }
}

#[derive(Clone, Serialize)]
pub struct PrintJob {
    pub id: u32,
    pub timestamp: u64,
    pub quantity: u32,
    #[serde(skip)]
    pub data: Option<GrayImage>,
    pub status: PrintJobStatus,
}

#[derive(Clone, Serialize)]
pub enum PrintJobStatus {
    Pending,
    Printing,
    Complete,
    Failed,
}

#[derive(Clone)]
pub enum PrintData {
    Image(GrayImage),
    Text(String),
}


#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
pub async fn rocket() -> _ {
    let queue = Arc::new(PrintQueue::new());
    let printing_queue = queue.clone();

    tokio::spawn(async move {
        //TODO recover from printer errors
        let mut printer = match printer::initial_printer() {
            Ok(printer) => printer,
            Err(_) => {
                eprintln!("Couldn't initialize printer.");
                process::exit(-1)
            }
        };
        loop {
            if printing_queue.jobs_todo.read().unwrap().len() == 0 {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }
            let next_job = printing_queue.jobs_todo.write().unwrap().pop_front();

            let next_job = match next_job {
                Some(job) => job,
                None => {
                    println!("No jobs in queue. Trying again in a second.");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
            };

            let mut failed = false;

            for _ in 0..next_job.quantity {
                match next_job.data {
                    None => continue,
                    Some(ref data) => {
                        match printer::print_image(data, &printer){
                            Ok(_) => {
                                println!("Printed sticker for job {}", next_job.id);
                            },
                            Err(e) => {
                                failed = true;
                                println!("Error printing job {}: {}", next_job.id, e);
                                printing_queue.jobs_other.write().unwrap().push_back(PrintJob{
                                    id: next_job.id,
                                    timestamp: next_job.timestamp,
                                    quantity: 1,
                                    data: None,
                                    status: PrintJobStatus::Failed
                                });
                                break;
                            }
                        }
                    }
                }
                if !failed{
                    printing_queue.jobs_other.write().unwrap().push_back(PrintJob{
                        id: next_job.id,
                        timestamp: next_job.timestamp,
                        quantity: 1,
                        data: None,
                        status: PrintJobStatus::Complete
                    });
                }
            }
        }
    });

    rocket::build().mount("/", routes![index, api::add_image_to_queue, api::add_text_to_queue, api::get_print_job, api::get_print_queue]).manage(queue.clone())
}