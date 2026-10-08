use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::Path,
};

use crate::{Image, Rgb, error::ImageError};

impl Image<Rgb<u8>> {
    /// Writes the image as a binary PPM (P6) file to `out`.
    pub fn write_ppm(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, "P6\n{} {}\n255\n", self.width, self.height)?;
        out.write_all(self.samples())?;
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

    /// Decodes a PGM or PPM image (P2, P3, P5 or P6) from `bytes`
    ///
    /// Gray images become RGB images with three equal samples per pixel.
    /// Samples are scaled from 0..=maxval to 0..=255, rounding to nearest.
    pub fn decode_pnm(bytes: &[u8]) -> Result<Self, ImageError> {
        let mut pos = 0;
        let header = parse_header(bytes, &mut pos)?;
        let samples = match header.encoding {
            Encoding::Plain => read_plain_raster(bytes, &mut pos, &header)?,
            Encoding::Binary => read_binary_raster(bytes, &mut pos, &header)?,
        };

        Ok(Image::from_samples(
            header.width,
            header.height,
            &to_rgb8(&samples, &header),
        ))
    }

    // Reads the file at `path` and decodes it with `decode_pnm`.
    pub fn load_pnm(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let bytes = std::fs::read(path)?;
        Image::decode_pnm(&bytes)
    }
}

/// How the samples are written: as decimal text, or as raw bytes.
#[derive(PartialEq)]
enum Encoding {
    Plain,
    Binary,
}

/// What the header of a PGM or PPM file tells us.
struct Header {
    encoding: Encoding,
    channels: usize, // 1 for PGM, 3 for PPM
    width: usize,
    height: usize,
    maxval: u16,
}

impl Header {
    /// How many samples the raster holds.
    fn sample_count(&self) -> usize {
        // parse_header checked that width * height * 6 doesn't overflow.
        self.width * self.height * self.channels
    }
}

/// The whitespace characters the Netpbm specification allows:
/// blanks, tabs, carriage returns and line feeds.
fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

/// Whether a token may end at `pos`: at the end of the data,
/// at whitespace, or where a comment starts.
fn at_delimeter(bytes: &[u8], pos: usize) -> bool {
    match bytes.get(pos) {
        None => true,
        Some(&byte) => is_whitespace(byte) || byte == b'#',
    }
}

/// Moves `pos` past a comment that starts at `pos`: everything from
/// the `#` up to, but not including, the next carriage return or line feed.
fn skip_comment(bytes: &[u8], pos: &mut usize) {
    while *pos < bytes.len() && bytes[*pos] != b'\n' && bytes[*pos] != b'\r' {
        *pos += 1;
    }
}

/// Moves `pos` past any mix of whitespace and comments.
fn skip_whitespace_and_comments(bytes: &[u8], pos: &mut usize) {
    while *pos < bytes.len() {
        if is_whitespace(bytes[*pos]) {
            *pos += 1;
        } else if bytes[*pos] == b'#' {
            skip_comment(bytes, pos);
        } else {
            break;
        }
    }
}

/// Skips the whitespace and comments, then reads a decimal number that must
/// end at a delimeter. `field` names the number in errors.
fn read_number(bytes: &[u8], pos: &mut usize, field: &'static str) -> Result<usize, ImageError> {
    skip_whitespace_and_comments(bytes, pos);
    if *pos == bytes.len() {
        return Err(ImageError::UnexpectedEof);
    }

    let start = *pos;
    let mut value: usize = 0;

    while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
        let digit = usize::from(bytes[*pos] - b'0');

        // Each digit shifts the number one decimal place to the left
        // and adds the digit.
        // e.g, reading 255 gives 2, then 25, then 255.
        let Some(shifted) = value.checked_mul(10) else {
            return Err(ImageError::BadNumber(field));
        };
        let Some(sum) = shifted.checked_add(digit) else {
            return Err(ImageError::BadNumber(field));
        };
        value = sum;
        *pos += 1;
    }
    if *pos == start || !at_delimeter(bytes, *pos) {
        return Err(ImageError::BadNumber(field));
    }

    Ok(value)
}

/// In a binary file, moves `pos` past the single whitespace character
/// that ends the header. A comment may come before it.
fn skip_raster_delimeter(bytes: &[u8], pos: &mut usize) -> Result<(), ImageError> {
    if bytes.get(*pos) == Some(&b'#') {
        skip_comment(bytes, pos);
    }
    if *pos == bytes.len() {
        return Err(ImageError::UnexpectedEof);
    }

    // read_number ended the maxval at a delimeter, and skip_comment stops
    // at a line break, so this byte is whitespace.
    *pos += 1;
    Ok(())
}

/// Reads the header from the start of `bytes` and leaves `pos`
/// at the first byte of the raster (binary formats)
/// or just after the maxval (plain formats)
fn parse_header(bytes: &[u8], pos: &mut usize) -> Result<Header, ImageError> {
    if bytes.len() < 2 {
        return Err(ImageError::UnexpectedEof);
    }
    let (encoding, channels) = match &bytes[..2] {
        b"P2" => (Encoding::Plain, 1),
        b"P3" => (Encoding::Plain, 3),
        b"P5" => (Encoding::Binary, 1),
        b"P6" => (Encoding::Binary, 3),
        _ => return Err(ImageError::UnknownFormat),
    };
    *pos = 2;

    // "P6" must be followed by whitespace or a comment, not by "P62".
    if !at_delimeter(bytes, *pos) {
        return Err(ImageError::UnknownFormat);
    }

    let width = read_number(bytes, pos, "width")?;
    let height = read_number(bytes, pos, "height")?;
    let maxval = read_number(bytes, pos, "maxval")?;
    if !(1..=65535).contains(&maxval) {
        return Err(ImageError::BadMaxval(maxval));
    }

    // The largest size we compute from the header is
    // width * height * 6: the raster of a 16-bit PPM, in bytes.
    // If that fits in a usize, so does every smaller size,
    // and later code can multiply freely.
    let Some(pixels) = width.checked_mul(height) else {
        return Err(ImageError::TooLarge { width, height });
    };
    if pixels.checked_mul(6).is_none() {
        return Err(ImageError::TooLarge { width, height });
    }

    if encoding == Encoding::Binary {
        skip_raster_delimeter(bytes, pos)?;
    }

    Ok(Header {
        encoding,
        channels,
        width,
        height,
        maxval: maxval as u16, // fits: we checked maxval <= 65535
    })
}

/// Reads the samples of a P% or P6 raster: one byte each if maxval < 256,
/// otherwise two bytes, most significant first.
fn read_binary_raster(
    bytes: &[u8],
    pos: &mut usize,
    header: &Header,
) -> Result<Vec<u16>, ImageError> {
    let count = header.sample_count();
    let bytes_per_sample = if header.maxval < 256 { 1 } else { 2 };

    // Check before allocating. A header can claim any size it likes, but the samples themselves have to be here.
    if bytes.len() - *pos < count * bytes_per_sample {
        return Err(ImageError::UnexpectedEof);
    }

    let mut samples = Vec::with_capacity(count);
    for _ in 0..count {
        let value = if bytes_per_sample == 1 {
            u16::from(bytes[*pos])
        } else {
            // max: 256 * 255 + 255, is 65535, u16
            256 * u16::from(bytes[*pos]) + u16::from(bytes[*pos + 1])
        };

        if value > header.maxval {
            return Err(ImageError::SampleOutOfRange {
                value: usize::from(value),
                maxval: usize::from(header.maxval),
            });
        }

        samples.push(value);
        *pos += bytes_per_sample;
    }

    Ok(samples)
}

/// Reads the samples of a P2 or P3 raster:
/// decimal numbers separated by whitespace
fn read_plain_raster(
    bytes: &[u8],
    pos: &mut usize,
    header: &Header,
) -> Result<Vec<u16>, ImageError> {
    let count = header.sample_count();

    // Every sample takes at least one byte, its digit.
    if bytes.len() - *pos < count {
        return Err(ImageError::UnexpectedEof);
    }

    let mut samples = Vec::with_capacity(count);
    for _ in 0..count {
        let value = read_number(bytes, pos, "sample")?;
        if value > usize::from(header.maxval) {
            return Err(ImageError::SampleOutOfRange {
                value,
                maxval: usize::from(header.maxval),
            });
        }
        samples.push(value as u16); // fits: value <= maxval <= 256
    }

    Ok(samples)
}

/// Turns the samples into 8-bit RGB: scales each ont to 0..=255 and,
/// for gray images, repeats it three times.
fn to_rgb8(samples: &[u16], header: &Header) -> Vec<u8> {
    let mut data = Vec::with_capacity(header.width * header.height * 3);

    for &sample in samples {
        let value = scale_to_u8(sample, header.maxval);
        if header.channels == 1 {
            data.extend_from_slice(&[value, value, value]);
        } else {
            data.push(value);
        }
    }

    data
}

/// Maps `sample` from 0..=maxval to 0..=255,
/// rounding to the nearest integer (halves round up).
/// Requires sample <= maxval
fn scale_to_u8(sample: u16, maxval: u16) -> u8 {
    let sample = u32::from(sample);
    let maxval = u32::from(maxval);

    // round(sample * 255 / maxval)
    // = floor((2 * sample * 255 + maxval) / (2 * maxval))
    ((2 * sample * 255 + maxval) / (2 * maxval)) as u8
}

#[cfg(test)]
mod ppm_tests {
    use super::*;

    #[test]
    fn header_is_exact() {
        let img: Image<Rgb<u8>> = Image::new(300, 2);
        let mut out = Vec::new();
        img.write_ppm(&mut out).unwrap();

        let header = b"P6\n300 2\n255\n";
        assert_eq!(&out[..header.len()], header);
        assert_eq!(out.len(), header.len() + 300 * 2 * 3);
    }

    #[test]
    fn pixel_bytes_land_at_their_offsets() {
        let mut img = Image::new(4, 3);
        img[(0, 0)] = [255, 0, 0];
        img[(3, 0)] = [0, 255, 0];
        img[(2, 1)] = [0, 0, 255];
        img[(3, 2)] = [7, 8, 9];
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
        img[(1, 0)] = [1, 2, 3];
        let mut expected = Vec::new();
        img.write_ppm(&mut expected).unwrap();

        let path = std::env::temp_dir().join(format!("imgkit-test-{}.ppm", std::process::id()));
        img.save_ppm(&path).unwrap();
        let written = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(written, expected);
    }
}
