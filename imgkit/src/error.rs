use std::{fmt, io};

/// Everything that can go wrong when reading or writing an image.
#[derive(Debug)]
pub enum ImageError {
    /// The operating system reported an error, such as a missing file.
    Io(io::Error),
    /// The data ended before the header or the pixels were complete.
    UnexpectedEof,
    /// The data doesn't start with a magic number we can read.
    UnknownFormat,
    /// The header field, or a sample in a plain file, isn't a decimal number.
    /// The `&'static str` names the field: "width", "height", "maxval" or "sample".
    BadNumber(&'static str),
    /// The maxval is outside 1..=65535
    BadMaxval(usize),
    /// The width and height are too large to compute the image's size.
    TooLarge { width: usize, height: usize },
    /// A sample is larger than the maxval.
    SampleOutOfRange { value: usize, maxval: usize },
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::Io(err) => write!(f, "I/O error: {err}"),
            ImageError::UnexpectedEof => write!(f, "the data ended before the image was complete"),
            ImageError::UnknownFormat => write!(f, "not a PGM or PPM file"),
            ImageError::BadNumber(field) => write!(f, "the {field} is not a valid number"),
            ImageError::BadMaxval(maxval) => {
                write!(f, "maxval {maxval} is outside the range 1 to 65535")
            }
            ImageError::TooLarge { width, height } => {
                write!(f, "a {width}x{height} image is too large")
            }
            ImageError::SampleOutOfRange { value, maxval } => {
                write!(f, "sample value {value} is larger than the maxval {maxval}")
            }
        }
    }
}

impl From<io::Error> for ImageError {
    fn from(err: io::Error) -> Self {
        ImageError::Io(err)
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn display_names_the_problem() {
        let err = ImageError::BadNumber("width");
        assert_eq!(err.to_string(), "the width is not a valid number");
        let err = ImageError::TooLarge {
            width: 4_000_000_000,
            height: 3,
        };
        assert_eq!(err.to_string(), "a 4000000000x3 image is too large");
    }

    #[test]
    fn io_errors_convert_with_from() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "no such file");
        let err = ImageError::from(io_err);
        assert_eq!(err.to_string(), "I/O error: no such file");
    }
}

#[cfg(test)]
mod decode_tests {
    use crate::Image;

    fn decode(bytes: &[u8]) -> Image {
        Image::decode_pnm(bytes).unwrap()
    }

    #[test]
    fn binary_ppm_round_trip() {
        let mut img = Image::new(5, 3);
        for y in 0..3 {
            for x in 0..5 {
                img.set_pixel(x, y, [x as u8 * 50, y as u8 * 100, 7]);
            }
        }
        let mut file = Vec::new();
        img.write_ppm(&mut file).unwrap();
        assert_eq!(decode(&file), img);
    }

    #[test]
    fn saved_file_loads_back() {
        let mut img = Image::new(3, 2);
        img.set_pixel(2, 1, [1, 2, 3]);
        let path = std::env::temp_dir().join(format!("imgkit-load-{}.ppm", std::process::id()));
        img.save_ppm(&path).unwrap();
        let loaded = Image::load_pnm(&path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(loaded.unwrap(), img);
    }

    #[test]
    fn plain_ppm() {
        let img = decode(b"P3\n2 1\n255\n255 0 0   0 128 255\n");
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 1);
        assert_eq!(img.get_pixel(0, 0), [255, 0, 0]);
        assert_eq!(img.get_pixel(1, 0), [0, 128, 255]);
    }

    #[test]
    fn gray_becomes_three_equal_samples() {
        let plain = decode(b"P2\n3 1\n255\n0 128 255\n");
        let binary = decode(b"P5\n3 1\n255\n\x00\x80\xff");
        for img in [plain, binary] {
            assert_eq!(img.get_pixel(0, 0), [0, 0, 0]);
            assert_eq!(img.get_pixel(1, 0), [128, 128, 128]);
            assert_eq!(img.get_pixel(2, 0), [255, 255, 255]);
        }
    }

    #[test]
    fn comments_and_any_whitespace_in_the_header() {
        let img = decode(
            b"P6#magic\n\t2 # width\r\n# a whole line\n1 255#maxval\n\x01\x02\x03\x04\x05\x06",
        );
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 1);
        assert_eq!(img.get_pixel(1, 0), [4, 5, 6]);
    }

    #[test]
    fn raster_may_start_with_whitespace_bytes() {
        // Exactly one whitespace byte ends the header. The newline, space,
        // tab and carriage return after it are samples 10, 32, 9 and 13.
        let img = decode(b"P6\n2 1\n255\n\n \t\r\n\n");
        assert_eq!(img.get_pixel(0, 0), [10, 32, 9]);
        assert_eq!(img.get_pixel(1, 0), [13, 10, 10]);
    }

    #[test]
    fn a_comment_after_the_delimiter_is_pixel_data() {
        // The space after the maxval ends the header, so "#!" is two samples.
        let img = decode(b"P5\n2 1\n255 #!");
        assert_eq!(img.get_pixel(0, 0), [b'#', b'#', b'#']);
        assert_eq!(img.get_pixel(1, 0), [b'!', b'!', b'!']);
    }

    #[test]
    fn other_maxvals_are_scaled_with_rounding() {
        // 255 * s / 1000 for s = 0, 1, 2, 500, 1000 is 0, 0.255, 0.51, 127.5, 255.
        let img = decode(b"P2\n5 1\n1000\n0 1 2 500 1000\n");
        assert_eq!(img.get_pixel(0, 0), [0, 0, 0]);
        assert_eq!(img.get_pixel(1, 0), [0, 0, 0]);
        assert_eq!(img.get_pixel(2, 0), [1, 1, 1]);
        assert_eq!(img.get_pixel(3, 0), [128, 128, 128]);
        assert_eq!(img.get_pixel(4, 0), [255, 255, 255]);

        let img = decode(b"P2\n2 1\n1\n0 1\n");
        assert_eq!(img.get_pixel(0, 0), [0, 0, 0]);
        assert_eq!(img.get_pixel(1, 0), [255, 255, 255]);
    }

    #[test]
    fn sixteen_bit_samples_are_big_endian() {
        // 0x1234 = 4660 -> 18.13 -> 18. Read little-endian, 0x3412 would give 52.
        // 0x00ff = 255 -> 0.99 -> 1, and 0x8000 = 32768 -> 127.50 -> 128.
        let img = decode(b"P6\n1 1\n65535\n\x12\x34\x00\xff\x80\x00");
        assert_eq!(img.get_pixel(0, 0), [18, 1, 128]);

        let img = decode(b"P3\n1 1\n65535\n65535 32768 0\n");
        assert_eq!(img.get_pixel(0, 0), [255, 128, 0]);
    }

    #[test]
    fn every_8_bit_value_survives_16_bits() {
        // 65535 = 255 * 257, so v * 257 is v rescaled to 16 bits exactly.
        let mut file = b"P5\n256 1\n65535\n".to_vec();
        for v in 0..=255u16 {
            let wide = v * 257;
            file.push((wide / 256) as u8); // most significant byte first
            file.push((wide % 256) as u8);
        }
        let img = decode(&file);
        for v in 0..=255u8 {
            assert_eq!(img.get_pixel(usize::from(v), 0), [v, v, v]);
        }
    }

    #[test]
    fn data_after_the_first_image_is_ignored() {
        let img = decode(b"P5\n1 1\n255\n\x07P5\n1 1\n255\n\x08");
        assert_eq!(img.get_pixel(0, 0), [7, 7, 7]);
    }
}

#[cfg(test)]
mod bad_input_tests {
    use crate::Image;

    use super::*;

    fn err(bytes: &[u8]) -> ImageError {
        Image::decode_pnm(bytes).unwrap_err()
    }

    #[test]
    fn unknown_magic_numbers() {
        assert!(matches!(err(b"P7\n2 2 255\n"), ImageError::UnknownFormat)); // PAM
        assert!(matches!(err(b"P4\n8 1\n\xff"), ImageError::UnknownFormat)); // PBM
        assert!(matches!(err(b"\x89PNG\r\n"), ImageError::UnknownFormat));
        assert!(matches!(err(b"p6\n1 1 255\n"), ImageError::UnknownFormat));
        assert!(matches!(err(b"P62 2 255\n"), ImageError::UnknownFormat));
    }

    #[test]
    fn header_ends_early() {
        for header in [
            &b""[..],
            b"P",
            b"P6",
            b"P6\n",
            b"P6\n2",
            b"P6\n2 2",
            b"P6\n2 2 255",
            b"P6\n2 2 255# a comment, but no newline",
            b"P6 # nothing but a comment",
        ] {
            assert!(matches!(err(header), ImageError::UnexpectedEof));
        }
    }

    #[test]
    fn fields_that_are_not_numbers() {
        assert!(matches!(
            err(b"P6\n-1 2 255\n"),
            ImageError::BadNumber("width")
        ));
        assert!(matches!(
            err(b"P6\n+1 2 255\n"),
            ImageError::BadNumber("width")
        ));
        assert!(matches!(
            err(b"P6\n2x2 255\n"),
            ImageError::BadNumber("width")
        ));
        assert!(matches!(
            err(b"P6\n2 2.5 255\n"),
            ImageError::BadNumber("height")
        ));
        assert!(matches!(
            err(b"P6\n2 2 ff\n"),
            ImageError::BadNumber("maxval")
        ));
        assert!(matches!(
            err(b"P6\n2 2 255x"),
            ImageError::BadNumber("maxval")
        ));
        assert!(matches!(
            err(b"P3\n1 1 255\n1 x 3\n"),
            ImageError::BadNumber("sample")
        ));
        assert!(matches!(
            err(b"P2\n1 1 255\n1e3\n"),
            ImageError::BadNumber("sample")
        ));
    }

    #[test]
    fn numbers_too_big_for_usize() {
        let header = b"P6\n99999999999999999999999 2 255\n";
        assert!(matches!(err(header), ImageError::BadNumber("width")));
    }

    #[test]
    fn maxval_out_of_range() {
        assert!(matches!(err(b"P6\n1 1 0\n"), ImageError::BadMaxval(0)));
        assert!(matches!(
            err(b"P6\n1 1 65536\n"),
            ImageError::BadMaxval(65536)
        ));
    }

    #[test]
    fn absurd_sizes_fail_before_allocating() {
        // 4e9 * 4e9 * 6 doesn't fit in 64 bits.
        let e = err(b"P6\n4000000000 4000000000 255\n\x00\x00\x00");
        assert!(matches!(
            e,
            ImageError::TooLarge {
                width: 4_000_000_000,
                height: 4_000_000_000
            }
        ));
        // 100,000 x 100,000 fits, but would need 30 GB of samples.
        let e = err(b"P6\n100000 100000 255\n\x00\x00\x00");
        assert!(matches!(e, ImageError::UnexpectedEof));
        let e = err(b"P3\n100000 100000 255\n0 0 0\n");
        assert!(matches!(e, ImageError::UnexpectedEof));
    }

    #[test]
    fn raster_ends_early() {
        assert!(matches!(
            err(b"P6\n2 2 255\n12345678901"),
            ImageError::UnexpectedEof
        ));
        assert!(matches!(
            err(b"P5\n1 1 65535\n\x01"),
            ImageError::UnexpectedEof
        ));
        assert!(matches!(
            err(b"P3\n1 1 255\n1 2"),
            ImageError::UnexpectedEof
        ));
    }

    #[test]
    fn samples_above_maxval() {
        assert!(matches!(
            err(b"P2\n1 1 100\n101\n"),
            ImageError::SampleOutOfRange {
                value: 101,
                maxval: 100
            }
        ));
        assert!(matches!(
            err(b"P5\n1 1 100\n\xc8"),
            ImageError::SampleOutOfRange {
                value: 200,
                maxval: 100
            }
        ));
        assert!(matches!(
            err(b"P5\n1 1 1000\n\x03\xe9"),
            ImageError::SampleOutOfRange {
                value: 1001,
                maxval: 1000
            }
        ));
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let result = Image::load_pnm("this/file/does/not/exist.ppm");
        let Err(ImageError::Io(io_err)) = result else {
            panic!("expected an I/O error, got {result:?}");
        };
        assert_eq!(io_err.kind(), io::ErrorKind::NotFound);
    }
}
