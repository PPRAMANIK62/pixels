mod error;
mod pnm;

pub use error::ImageError;

/// An image made of 8-bit RGB pixels, stored row by row.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    width: usize,
    height: usize,
    data: Vec<u8>,
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
