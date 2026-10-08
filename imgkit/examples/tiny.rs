use std::io;

use imgkit::Image;

fn main() -> io::Result<()> {
    let mut img = Image::new(3, 2);
    img[(0, 0)] = [255, 0, 0]; // red
    img[(1, 0)] = [0, 255, 0]; // green
    img[(2, 0)] = [0, 0, 255]; // blue
    img[(0, 1)] = [255, 255, 0]; // yellow
    img[(1, 1)] = [255, 255, 255]; // white
    // (2, 1) stays black
    img.save_ppm("tiny.ppm")?;
    Ok(())
}
