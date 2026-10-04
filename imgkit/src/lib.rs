use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::Path,
};

/// An image made of 8-bit RGB pixels, stored row by row.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl Image {
    /// Create a black image of the given size.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0; width * height * 3],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the `[r, g, b]` color of the pixel at column `x`, row `y`.
    pub fn get_pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let i = self.offset(x, y);
        [self.data[i], self.data[i + 1], self.data[i + 2]]
    }

    /// Sets the pixel at column `x`, row `y` to the color `rgb`.
    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: [u8; 3]) {
        let i = self.offset(x, y);
        self.data[i] = rgb[0];
        self.data[i + 1] = rgb[1];
        self.data[i + 2] = rgb[2];
    }

    /// Where the red byte of pixel (x, y) lives in `data`.
    /// Panics if (x, y) is outside the image.
    fn offset(&self, x: usize, y: usize) -> usize {
        assert!(
            x < self.width && y < self.height,
            "pixel ({x}, {y}) is outside the {}x{} image",
            self.width,
            self.height
        );
        (y * self.width + x) * 3
    }

    /// Writes the image as a binary PPM (P6) file to `out`.
    pub fn write_ppm(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, "P6\n{} {}\n255\n", self.width, self.height)?;
        out.write_all(&self.data)?;
        Ok(())
    }

    /// Creates (or replaces) the file at `path` and writes the image to it as PPM.
    pub fn save_ppm(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let file = File::create(path)?;
        let mut out = BufWriter::new(file);
        self.write_ppm(&mut out)?;
        out.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod pixel_tests {
    use super::*;

    #[test]
    fn new_image_is_black() {
        let img = Image::new(4, 3);
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 3);
        for y in 0..3 {
            for x in 0..4 {
                assert_eq!(img.get_pixel(x, y), [0, 0, 0]);
            }
        }
    }

    #[test]
    fn set_pixel_then_get_pixel() {
        let mut img = Image::new(4, 3);
        img.set_pixel(2, 1, [10, 20, 30]);
        assert_eq!(img.get_pixel(2, 1), [10, 20, 30]);
        // The pixel with x and y swapped is untouched.
        assert_eq!(img.get_pixel(1, 2), [0, 0, 0]);
    }

    #[test]
    fn pixels_are_stored_row_by_row() {
        let mut img = Image::new(2, 2);
        img.set_pixel(1, 0, [1, 2, 3]);
        img.set_pixel(0, 1, [4, 5, 6]);
        // Pixels in order: (0, 0), (1, 0), (0, 1), (1, 1).
        assert_eq!(img.data, vec![0, 0, 0, 1, 2, 3, 4, 5, 6, 0, 0, 0]);
    }

    #[test]
    #[should_panic(expected = "pixel (4, 0) is outside the 4x3 image")]
    fn get_pixel_past_the_right_edge_panics() {
        let img = Image::new(4, 3);
        img.get_pixel(4, 0);
    }

    #[test]
    #[should_panic(expected = "pixel (0, 3) is outside the 4x3 image")]
    fn set_pixel_below_the_bottom_edge_panics() {
        let mut img = Image::new(4, 3);
        img.set_pixel(0, 3, [255, 255, 255]);
    }
}

#[cfg(test)]
mod ppm_tests {
    use super::*;

    #[test]
    fn header_is_exact() {
        let img = Image::new(300, 2);
        let mut out = Vec::new();
        img.write_ppm(&mut out).unwrap();

        let header = b"P6\n300 2\n255\n";
        assert_eq!(&out[..header.len()], header);
        assert_eq!(out.len(), header.len() + 300 * 2 * 3);
    }

    #[test]
    fn pixel_bytes_land_at_their_offsets() {
        let mut img = Image::new(4, 3);
        img.set_pixel(0, 0, [255, 0, 0]);
        img.set_pixel(3, 0, [0, 255, 0]);
        img.set_pixel(2, 1, [0, 0, 255]);
        img.set_pixel(3, 2, [7, 8, 9]);
        let mut out = Vec::new();
        img.write_ppm(&mut out).unwrap();

        // The header "P6\n4 3\n255\n" is 11 bytes long.
        // Pixel (x, y) starts (y * 4 + x) * 3 bytes after it.
        let start = 11;
        assert_eq!(out[start..start + 3], [255, 0, 0]); // (0, 0): offset 0
        assert_eq!(out[start + 9..start + 12], [0, 255, 0]); // (3, 0): offset 9
        assert_eq!(out[start + 18..start + 21], [0, 0, 255]); // (2, 1): offset 18
        assert_eq!(out[start + 33..], [7, 8, 9]); // (3, 2): offset 33, the last pixel
    }

    #[test]
    fn save_ppm_writes_the_same_bytes_to_a_file() {
        let mut img = Image::new(2, 1);
        img.set_pixel(1, 0, [1, 2, 3]);
        let mut expected = Vec::new();
        img.write_ppm(&mut expected).unwrap();

        let path = std::env::temp_dir().join(format!("imgkit-test-{}.ppm", std::process::id()));
        img.save_ppm(&path).unwrap();
        let written = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(written, expected);
    }
}
