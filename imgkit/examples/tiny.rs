use std::io;

use imgkit::Image;

fn main() -> io::Result<()> {
    let mut img = Image::new(3, 2);
    img.set_pixel(0, 0, [255, 0, 0]); // red
    img.set_pixel(1, 0, [0, 255, 0]); // green
    img.set_pixel(2, 0, [0, 0, 255]); // blue
    img.set_pixel(0, 1, [255, 255, 0]); // yellow
    img.set_pixel(1, 1, [255, 255, 255]); // white
    // (2, 1) stays black
    img.save_ppm("tiny.ppm")?;
    Ok(())
}
