use imageproc::drawing::draw_text_mut;
use rusttype::Scale;
use rusttype::Font;
use brother_ql_rs::printer;
use brother_ql_rs::printer::ThermalPrinter;
use image::{DynamicImage, GrayImage, Luma};
use imageproc::drawing::text_size;
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
}

pub fn print_image(image: &GrayImage, printer: &ThermalPrinter<rusb::GlobalContext>) -> brother_ql_rs::printer::Result<()> {
    let rasterized = rasterise_image(image).unwrap();
    printer.print_blocking(rasterized)
}

pub enum FontVariants{
    GabriellaHeavy,
    MonoSans,
    Arial,
    ArialBold,
    ArialItalic,
    ArialBoldItalic,
}

impl FontVariants{
    pub fn from_str(string: &str) -> Option<FontVariants>{
        match string{
            "GabriellaHeavy" => Some(FontVariants::GabriellaHeavy),
            "MonoSans" => Some(FontVariants::MonoSans),
            "Arial" => Some(FontVariants::Arial),
            "ArialBold" => Some(FontVariants::ArialBold),
            "ArialItalic" => Some(FontVariants::ArialItalic),
            "ArialBoldItalic" => Some(FontVariants::ArialBoldItalic),
            _ => None
        }
    }
    pub fn get_font(&self) -> Font{
        match self{
            FontVariants::GabriellaHeavy => {
                let font = Vec::from(include_bytes!("../fonts/GabriellaHeavy.otf") as &[u8]);
                Font::try_from_vec(font).unwrap()
            },
            FontVariants::MonoSans => {
                let font = Vec::from(include_bytes!("../fonts/Mona-Sans.ttf") as &[u8]);
                Font::try_from_vec(font).unwrap()
            }
            FontVariants::Arial => {
                let font = Vec::from(include_bytes!("../fonts/arial.ttf") as &[u8]);
                Font::try_from_vec(font).unwrap()
            }
            FontVariants::ArialBold => {
                let font = Vec::from(include_bytes!("../fonts/arialbd.ttf") as &[u8]);
                Font::try_from_vec(font).unwrap()
            }
            FontVariants::ArialItalic => {
                let font = Vec::from(include_bytes!("../fonts/ariali.ttf") as &[u8]);
                Font::try_from_vec(font).unwrap()
            }
            FontVariants::ArialBoldItalic => {
                let font = Vec::from(include_bytes!("../fonts/arialbi.ttf") as &[u8]);
                Font::try_from_vec(font).unwrap()
            }
        }
    }
}

/// Converts a text to an image
/// # Arguments
/// * `text` - The text to convert, each line separated by a newline character
/// * `length` - The max length of the paper in pixels. If rotate is true (portrait mode), this is handled as max height, else it is exactly the length
/// * `rotate` - If true, the image is rotated by 90° (portrait mode)
/// * `invert` - If true, the image is inverted (white text on black background)
/// * `font` - The font to use
pub fn text_to_image(text: String, length: u32, rotate: bool, invert: bool, font: FontVariants) -> Result<GrayImage, ()>{
    let height = 704;
    let lines: Vec<String> = text.lines().map(|line| line.to_string()).collect();

    // Figure out which line is the longest
    let mut longest_length = 0;
    let mut longest_line = 0;
    for line in 0..lines.len(){
        let (length, _) = text_size(Scale{x: 10.0, y: 10.0}, &font.get_font(), &lines.get(line).unwrap());
        if length > longest_length{
            longest_length = length;
            longest_line = line;
        }
    }

    let longest_line = lines.get(longest_line).unwrap();

    //Figure out maximal fitting font size
    let mut scale = Scale {
        x: 1.0,
        y: 1.0,
    };

    let mut font_size = 1;
    let mut line_height = 0;

    loop {
        let test_scale = Scale {
            x: font_size as f32,
            y: font_size as f32,
        };
        println!("Trying font scale {}", font_size);
        let (text_width, text_height) = text_size(test_scale, &font.get_font(), &longest_line);

        if rotate{
            // portrait mode
            // Check if text width fits in paper height && text height fits in paper length
            if text_width < height as i32 && (text_height*(lines.len() as i32)+10*(lines.len() as i32)) < length as i32{
                scale = test_scale;
                line_height = text_height;
                font_size += 1;
            }else{
                break;
            }
        }else{
            // landscape mode
            // Check if text width fits in paper length && text height fits in paper height
            if text_width < length as i32 && (text_height*(lines.len() as i32)+10*(lines.len() as i32)) < height as i32 {
                scale = test_scale;
                line_height = text_height;
                font_size += 1;
            }else{
                break;
            }
        }
    }

    let font_color = if invert{
        Luma([0xFF])
    }else{
        Luma([0x00])
    };

    let mut image = if rotate{
        //Rotated (portrait)
        let needed_length = line_height*(lines.len() as i32)+10*(lines.len() as i32);
        DynamicImage::new_luma8(height, needed_length as u32).to_luma8()
    }else{
        //Default mode (landscape) y = height = 704
        DynamicImage::new_luma8(length, height).to_luma8()
    };
    //Set background:
    if invert{ // Black background
        for pixel in image.pixels_mut(){
            *pixel = Luma([0x00]);
        }
    }else{// White background
        for pixel in image.pixels_mut(){
            *pixel = Luma([0xFF]);
        }
    }

    let mut y = 0;
    //Draw text
    for line in lines{
        draw_text_mut(&mut image, font_color, 0, y, scale, &font.get_font(), &line);
        y += (line_height + 10) as i32;
    }

    if rotate{
        image = image::imageops::rotate270(&mut image);
    }

    Ok(image)
}

pub fn rasterise_image(image: &GrayImage) -> Result<Vec<[u8; 90]>, ()> {
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
        for byte in line[1..89].iter_mut(){
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
