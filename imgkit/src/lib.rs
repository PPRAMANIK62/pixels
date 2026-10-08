mod error;
mod pixel;
mod pnm;

use std::ops::{Index, IndexMut};

pub use error::ImageError;
pub use pixel::{Luma, LumaA, Rgb, Rgba, Sample};

use crate::pixel::Pixel;

/// An rectangle of pixels of type `P`, stored row by row with no gaps.
#[derive(Debug, Clone, PartialEq)]
pub struct Image<P> {
    width: usize,
    height: usize,
    pixels: Vec<P>,
}

/// `width * height`, or a panic if that doesn't fit in a `usize`.
fn pixel_count(width: usize, height: usize) -> usize {
    let Some(count) = width.checked_mul(height) else {
        panic!("a {width}x{height} image is too large");
    };
    count
}

/// Where pixel (x, y) lives in a slice whose rows start `stride` pixels apart.
/// Panics if (x, y) is outside the `width` x `height` rectangle.
fn pixel_offset(x: usize, y: usize, width: usize, height: usize, stride: usize) -> usize {
    assert!(
        x < width && y < height,
        "pixel ({x}, {y}) is outside the {width}x{height} image"
    );
    y * stride + x
}

impl<P: Copy> Image<P> {
    /// Creates an image with every pixel set to `pixel`.
    pub fn filled(width: usize, height: usize, pixel: P) -> Self {
        let pixels = vec![pixel; pixel_count(width, height)];
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Creates an image from it's pixels, in row-major order.
    /// Panics if there aren't exactly `width * height` of them.
    pub fn from_pixels(width: usize, height: usize, pixels: Vec<P>) -> Self {
        assert_eq!(
            pixels.len(),
            pixel_count(width, height),
            "a {width}x{height} image needs {} pixels",
            pixel_count(width, height)
        );
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Creates an image by calling `f(x, y) for every pixel, row by row.`
    pub fn from_fn(width: usize, height: usize, mut f: impl FnMut(usize, usize) -> P) -> Self {
        let mut pixels = Vec::with_capacity(pixel_count(width, height));
        for y in 0..height {
            for x in 0..width {
                pixels.push(f(x, y));
            }
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// All pixels, row by row.
    pub fn pixels(&self) -> &[P] {
        &self.pixels
    }

    /// All pixels, row by row, for changing them.
    pub fn pixels_mut(&mut self) -> &mut [P] {
        &mut self.pixels
    }

    /// The pixel at (x, y), or `None` is (x, y) is outside the image.
    pub fn get(&self, x: usize, y: usize) -> Option<&P> {
        if x < self.width && y < self.height {
            Some(&self.pixels[y * self.width + x])
        } else {
            None
        }
    }

    /// Row `y`, `width` pixels long. Panics if `y` is outside the image.
    pub fn row(&self, y: usize) -> &[P] {
        assert!(
            y < self.height,
            "row {y} is outside the {}x{} image",
            self.width,
            self.height
        );

        let start = y * self.width;
        &self.pixels[start..start + self.width]
    }

    /// Row `y`, for changing it. Panics if `y` is outside the image.
    pub fn row_mut(&mut self, y: usize) -> &mut [P] {
        assert!(
            y < self.height,
            "row {y} is outside the {}x{} image",
            self.width,
            self.height
        );
        let start = y * self.width;
        &mut self.pixels[start..start + self.width]
    }

    /// The rows, top to bottom. An image without pixels has no rows.
    pub fn rows(&self) -> impl Iterator<Item = &[P]> {
        // chunks_exact(0) would panic, with no pixels there are no chunks anyway.
        // so any nonzero size gives the same (empty) result.
        self.pixels.chunks_exact(self.width.max(1))
    }

    /// The rows, top to bottom, for chaning them.
    pub fn rows_mut(&mut self) -> impl Iterator<Item = &mut [P]> {
        self.pixels.chunks_exact_mut(self.width.max(1))
    }
}

impl<P: Copy> Index<(usize, usize)> for Image<P> {
    type Output = P;

    /// `image[(x, y)]` is the pixel at column `x`, row `y`.
    /// Panics if (x, y) is outside the image.
    fn index(&self, (x, y): (usize, usize)) -> &P {
        let i = pixel_offset(x, y, self.width, self.height, self.width);
        &self.pixels[i]
    }
}

impl<P: Copy> IndexMut<(usize, usize)> for Image<P> {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut P {
        let i = pixel_offset(x, y, self.width, self.height, self.width);
        &mut self.pixels[i]
    }
}

impl<P: Pixel> Image<P> {
    /// Creates an image whose samples are all zero:
    /// black, and fully transparent if `P` has alpha
    pub fn new(width: usize, height: usize) -> Self {
        Self::filled(width, height, P::default())
    }

    /// Converts every pixel to the pixel type `Q`
    pub fn convert<Q: Pixel>(&self) -> Image<Q> {
        let mut pixels = Vec::with_capacity(self.pixels.len());
        for &pixel in &self.pixels {
            pixels.push(Q::from_rgba(pixel.to_rgba()));
        }

        Image {
            width: self.width,
            height: self.height,
            pixels,
        }
    }
}

impl<T: Sample, const N: usize> Image<[T; N]> {
    /// Creates an image from interleaved samples, `N` per pixel, row by row.
    /// Panics if there arn't exactly `width * height * N` of them.
    pub fn from_samples(width: usize, height: usize, samples: &[T]) -> Self {
        let (pixels, rest) = samples.as_chunks::<N>();
        assert!(
            rest.is_empty() && pixels.len() == pixel_count(width, height),
            "a {width}x{height} image needs {N} samples per pixel, got {} samples",
            samples.len()
        );

        Self {
            width,
            height,
            pixels: pixels.to_vec(),
        }
    }

    /// All samples, interleved, row by row.
    pub fn samples(&self) -> &[T] {
        self.pixels.as_flattened()
    }

    /// All samples, interleved, row by row, for changing them.
    pub fn samples_mut(&mut self) -> &mut [T] {
        self.pixels.as_flattened_mut()
    }

    /// Gives up the image and returns it's samples, without copying them.
    pub fn into_samples(self) -> Vec<T> {
        self.pixels.into_flattened()
    }
}

#[cfg(test)]
mod image_tests {
    use super::*;

    #[test]
    fn new_image_is_black() {
        let img: Image<Rgb<u8>> = Image::new(4, 3);
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 3);
        for y in 0..3 {
            for x in 0..4 {
                assert_eq!(img[(x, y)], [0, 0, 0]);
            }
        }
        let img: Image<Rgba<f32>> = Image::new(2, 2);
        assert_eq!(img[(1, 1)], [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn write_with_index_then_read() {
        let mut img: Image<Rgb<u8>> = Image::new(4, 3);
        img[(2, 1)] = [10, 20, 30];
        assert_eq!(img[(2, 1)], [10, 20, 30]);
        // The pixel with x and y swapped is untouched.
        assert_eq!(img[(1, 2)], [0, 0, 0]);
        // A pixel is an array, so one sample can be changed on its own.
        img[(2, 1)][1] = 99;
        assert_eq!(img[(2, 1)], [10, 99, 30]);
    }

    #[test]
    fn pixels_are_stored_row_by_row() {
        let mut img: Image<Rgb<u8>> = Image::new(2, 2);
        img[(1, 0)] = [1, 2, 3];
        img[(0, 1)] = [4, 5, 6];
        // Pixels in order: (0, 0), (1, 0), (0, 1), (1, 1).
        assert_eq!(img.pixels(), [[0, 0, 0], [1, 2, 3], [4, 5, 6], [0, 0, 0]]);
        // The same memory, seen as interleaved samples.
        assert_eq!(img.samples(), [0, 0, 0, 1, 2, 3, 4, 5, 6, 0, 0, 0]);
    }

    #[test]
    #[should_panic(expected = "pixel (4, 0) is outside the 4x3 image")]
    fn reading_past_the_right_edge_panics() {
        let img: Image<Luma<u8>> = Image::new(4, 3);
        let _ = img[(4, 0)];
    }

    #[test]
    #[should_panic(expected = "pixel (0, 3) is outside the 4x3 image")]
    fn writing_below_the_bottom_edge_panics() {
        let mut img: Image<Luma<u8>> = Image::new(4, 3);
        img[(0, 3)] = [255];
    }

    #[test]
    fn get_returns_none_outside_the_image() {
        let img = Image::filled(4, 3, [7u8]);
        assert_eq!(img.get(3, 2), Some(&[7]));
        assert_eq!(img.get(4, 0), None);
        assert_eq!(img.get(0, 3), None);
    }

    #[test]
    fn from_fn_calls_f_for_every_pixel() {
        let img = Image::from_fn(3, 2, |x, y| [x as u16 * 10 + y as u16]);
        assert_eq!(img.pixels(), [[0], [10], [20], [1], [11], [21]]);
    }

    #[test]
    fn rows_are_width_pixels_long() {
        let mut img = Image::from_fn(3, 2, |x, y| [x as u8, y as u8]);
        assert_eq!(img.row(1), [[0, 1], [1, 1], [2, 1]]);
        let rows: Vec<&[LumaA<u8>]> = img.rows().collect();
        assert_eq!(rows, [img.row(0), img.row(1)]);

        img.row_mut(0)[2] = [9, 9];
        for row in img.rows_mut() {
            row[0] = [5, 5];
        }
        assert_eq!(
            img.pixels(),
            [[5, 5], [1, 0], [9, 9], [5, 5], [1, 1], [2, 1]]
        );
    }

    #[test]
    #[should_panic(expected = "row 2 is outside the 3x2 image")]
    fn row_below_the_image_panics() {
        let img: Image<Rgb<u8>> = Image::new(3, 2);
        img.row(2);
    }

    #[test]
    fn images_without_pixels() {
        let mut img: Image<Rgb<u8>> = Image::new(0, 3);
        assert_eq!(img.pixels().len(), 0);
        assert_eq!(img.rows().count(), 0);
        assert_eq!(img.rows_mut().count(), 0);
        assert_eq!(img.get(0, 0), None);
    }

    #[test]
    fn samples_round_trip() {
        let samples = [1u16, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let img: Image<Rgb<u16>> = Image::from_samples(2, 2, &samples);
        assert_eq!(img[(1, 0)], [4, 5, 6]);
        let img: Image<Rgba<u16>> = Image::from_samples(3, 1, &samples);
        assert_eq!(img[(1, 0)], [5, 6, 7, 8]);
        assert_eq!(img.into_samples(), samples);
    }

    #[test]
    #[should_panic(expected = "a 2x2 image needs 3 samples per pixel, got 11 samples")]
    fn from_samples_checks_the_count() {
        let _: Image<Rgb<u8>> = Image::from_samples(2, 2, &[0; 11]);
    }

    #[test]
    #[should_panic(expected = "a 4000000000x5000000000 image is too large")]
    fn sizes_that_overflow_panic() {
        let _: Image<Luma<u8>> = Image::new(4_000_000_000, 5_000_000_000);
    }
}

#[cfg(test)]
mod convert_tests {
    use super::*;

    #[test]
    fn rgb8_survives_a_trip_through_f32() {
        let img = Image::from_fn(16, 16, |x, y| [(x * 16) as u8, (y * 16) as u8, 255]);
        let float: Image<Rgb<f32>> = img.convert();
        assert_eq!(float.width(), 16);
        assert_eq!(float[(1, 0)], [16.0 / 255.0, 0.0, 1.0]);
        assert_eq!(float.convert::<Rgb<u8>>(), img);
    }

    #[test]
    fn gray_conversion_keeps_the_size() {
        let img: Image<Rgb<u8>> = Image::filled(5, 2, [0, 255, 0]);
        let gray = img.convert::<Luma<u8>>();
        assert_eq!((gray.width(), gray.height()), (5, 2));
        assert_eq!(gray[(4, 1)], [182]);
    }

    #[test]
    fn eight_bits_to_sixteen_and_back() {
        let img = Image::from_fn(256, 1, |x, _| [x as u8]);
        let deep: Image<Luma<u16>> = img.convert();
        assert_eq!(deep[(255, 0)], [65535]);
        assert_eq!(deep[(1, 0)], [257]);
        assert_eq!(deep.convert::<Luma<u8>>(), img);
    }

    #[test]
    fn premultiply_a_whole_image() {
        let mut img: Image<Rgba<f32>> = Image::filled(2, 1, [1.0, 0.5, 0.25, 0.5]);
        img.premultiply_alpha();
        assert_eq!(img[(1, 0)], [0.5, 0.25, 0.125, 0.5]);
        img.unpremultiply_alpha();
        assert_eq!(img[(0, 0)], [1.0, 0.5, 0.25, 0.5]);
    }
}
