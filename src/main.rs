use std::collections::VecDeque;
use std::process;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, RwLock};
use image::{DynamicImage, GrayImage};
use rocket::tokio;

#[macro_use] extern crate rocket;

pub mod api;
pub mod printer;

pub struct PrintQueue{
    pub jobs_todo: RwLock<VecDeque<PrintJob>>,
    pub jobs_other: RwLock<VecDeque<PrintJob>>,
    counter: AtomicU32,
}

impl PrintQueue{
    pub fn new() -> PrintQueue{
        PrintQueue{
            jobs_todo: Default::default(),
            counter: AtomicU32::new(0),
            jobs_other: Default::default(),
        }
    }
    pub fn add_job(&self, timestamp: u64, data: PrintData, quantity: u32) -> u32{
        let id = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)+1;
        let print_job = PrintJob{
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

pub struct PrintJob{
    pub id: u32,
    pub timestamp: u64,
    pub quantity: u32,
    pub data: Option<PrintData>,
    pub status: PrintJobStatus
}

pub enum PrintJobStatus{
    Pending,
    Printing,
    Complete,
    Failed
}

pub enum PrintData{
    Image(GrayImage),
    Text(String)
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
        let mut printer = match printer::initial_printer(){
            Ok(printer) => printer,
            Err(_) => {
                eprintln!("Couldn't initialize printer.");
                process::exit(-1)
            }
        };
        loop{
            if printing_queue.jobs_todo.read().unwrap().len() == 0{
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }
            let next_job = printing_queue.jobs_todo.write().unwrap().pop_front();

            let next_job = match next_job{
                Some(job) => job,
                None => {
                    println!("No jobs in queue. Trying again in a second.");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
            };

            for _ in 0..next_job.quantity{
                match next_job.data{
                    None => continue,
                    Some(ref data) => {
                        match data{
                            PrintData::Image(img) => {
                                match printer::print_image(img, &printer){
                                    Ok(_) => {
                                        println!("Printed job {}", next_job.id);
                                        printing_queue.jobs_other.write().unwrap().push_back(PrintJob{
                                            id: next_job.id,
                                            timestamp: next_job.timestamp,
                                            quantity: next_job.quantity,
                                            data: None,
                                            status: PrintJobStatus::Complete
                                        });
                                    },
                                    Err(e) => {
                                        println!("Failed to print job {}: {}", next_job.id, e);

                                        printing_queue.jobs_other.write().unwrap().push_back(PrintJob{
                                            id: next_job.id,
                                            timestamp: next_job.timestamp,
                                            quantity: next_job.quantity,
                                            data: None,
                                            status: PrintJobStatus::Failed
                                        });

                                        //Try to re-initialize printer and try again
                                        printer = match printer::initial_printer(){
                                            Ok(printer) => printer,
                                            Err(_) => {
                                                eprintln!("Couldn't re-initialize printer. That's not good.");
                                                continue
                                            }
                                        };
                                    }
                                }
                            }
                            PrintData::Text(_) => {
                                //unimplemented
                                continue;
                            }
                        }
                    }
                }
            }
        }
    });

    rocket::build().mount("/", routes![index, api::add_image_to_queue]).manage(queue.clone())
}