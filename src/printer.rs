use std::path::PathBuf;
use brother_ql_rs::printer;
use brother_ql_rs::printer::ThermalPrinter;
use image::{DynamicImage, GrayImage, Luma};
use image::imageops::FilterType;

pub fn initial_printer() -> Result<ThermalPrinter<rusb::GlobalContext>, ()>{
    let binding = match printer::printers().pop() {
        Some(binding) => binding,
        None => {
            eprintln!("No printer found!");
            return Err(())
        }
    };
    match ThermalPrinter::new(binding){
        Ok(printer) => Ok(printer),
        Err(e) => {
            eprintln!("Could not initialize printer: {}", e);
            Err(())
        }
    }
    //nheight HAS TO BE 704 PIXELS (~ 62mm) !!!!
    //let image = image::open(PathBuf::from("test.png")).unwrap().grayscale().resize_exact(704, 704, FilterType::Lanczos3).to_luma8();

    //let to_print = rasterize_image(&image).unwrap();
    //printer.print(to_print).unwrap();
}

pub fn print_image(image: &GrayImage, printer: &ThermalPrinter<rusb::GlobalContext>) -> brother_ql_rs::printer::Result<()> {
    let rasterized = rasterize_image(image).unwrap();
    printer.print_blocking(rasterized)
}

pub fn rasterize_image(image: &GrayImage) -> Result<Vec<[u8; 90]>, ()> {
    let length = image.width(); // = x
    let height = image.height(); // = y

    // Height has to be exactly 704 pixel (pins)
    if height != 704{
        eprintln!("Image height has to be exactly 704 pixels!");
        return Err(());
    }

    let mut lines = Vec::with_capacity(length as usize);
    // Number of lines equals the length of the image
    for x in 0..length{
        let mut line = [0; 90];
        let mut y = 0;

        // Set first empty byte to reflect margins
        line[0] = 0x00;

        //A line is 90 bytes long
        for mut byte in line[1..89].iter_mut(){
            let mut new_byte: u8= 0;

            // One byte = 8 bits
            for bitindex in 0..8{
                if y >= height{
                    break;
                }
                let next_pixel = image.get_pixel(x, y);
                let value: u8 = if next_pixel[0] > 0xFF / 2 {
                    0
                }
                else {
                    1
                };
                new_byte |= value << bitindex;
                y += 1;

            }
            // We need the byte in reverse order for the printer
            *byte = new_byte.reverse_bits();

        }

        // Set last empty byte to reflect margins
        line[89] = 0x00;

        lines.push(line);
    }

    Ok(lines)
}

pub fn image_to_raster_lines_legacy(image: &image::GrayImage) -> Vec<[u8; 90]> {
    let width = image.width() as usize;
    let line_count = image.len() / width;

    // We need to sidescan this generated image for the printer
    let mut lines = Vec::with_capacity(width);
    for c in 0..width {
        let mut line = [0; 90]; // Always 90 for regular sized printers like the QL-700 (with a 0x00 byte to start)
        let mut line_byte = 1;
        // Bit index counts backwards
        // First nibble (bits 7 through 4) in the second byte is blank
        let mut line_bit_index: i8 = 3;
        for r in 0..line_count {
            line_bit_index -= 1;
            if line_bit_index < 0 {
                line_byte += 1;
                line_bit_index += 8;
            }
            image.get_pixel(0, 0);
            let luma_pixel = image.get_pixel(c as u32, r as u32);
            let value: u8 = if luma_pixel[0] > 0xFF / 2 {
                0
            }
            else {
                1
            };
            line[line_byte] |= value << line_bit_index;
        }
        lines.push(line);
    }
    lines
}