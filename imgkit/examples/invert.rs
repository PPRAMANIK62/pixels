use imgkit::{Image, ImageError};

fn main() -> Result<(), ImageError> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: cargo run --example invert -- INPUT OUTPUT.ppm");
        return Ok(());
    }

    let input = &args[1];
    let output = &args[2];

    let mut img = Image::load_pnm(input)?;
    for y in 0..img.height() {
        for x in 0..img.width() {
            let [r, g, b] = img[(x, y)];
            img[(x, y)] = [255 - r, 255 - g, 255 - b];
        }
    }
    img.save_ppm(output)?;

    println!(
        "inverted {input} ({}x{}) into {output}",
        img.width(),
        img.height()
    );

    Ok(())
}
