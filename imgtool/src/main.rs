use imgkit::Image;
use std::io::Result;

fn main() -> Result<()> {
    let mut img = Image::new(256, 256);
    for y in 0..256 {
        for x in 0..256 {
            // Red grows from left to write, blue from top to bottom.
            img.set_pixel(x, y, [x as u8, 0, y as u8]);
        }
    }
    img.save_ppm("gradient.ppm")?;
    println!("wrote gradient.ppm ({}x{})", img.width(), img.height());
    Ok(())
}
