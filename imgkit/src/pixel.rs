use std::fmt::Debug;

/// A number type that holds one sample of a pixel: `u8`, `u16` or `f32`.
///
/// Integer samples use their whole range,
/// from 0 (none of that channel) to `MAX` (all of it).
/// `f32` samples use 0.0 to 1.0 for the same range, and may go outside it.
pub trait Sample: Copy + Default + PartialEq + PartialOrd + Debug + Send + Sync {
    /// The sample value that means "all of this channel": 255, 65535, or 1.0.
    const MAX: Self;

    /// Converts to `f32`, mapping 0..=MAX to 0.0..=1.0.
    fn to_f32(self) -> f32;

    /// Converts from `f32`, mapping 0.0..=1.0 to 0..=MAX.
    /// Integer types round to the nearest value and clamp values outside the range.
    fn from_f32(value: f32) -> Self;
}

impl Sample for u8 {
    const MAX: u8 = 255;

    fn to_f32(self) -> f32 {
        f32::from(self) / 255.0
    }

    fn from_f32(value: f32) -> Self {
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    }
}

impl Sample for u16 {
    const MAX: u16 = 65535;

    fn to_f32(self) -> f32 {
        f32::from(self) / 65535.0
    }

    fn from_f32(value: f32) -> Self {
        (value.clamp(0.0, 1.0) * 65535.0).round() as u16
    }
}

impl Sample for f32 {
    const MAX: f32 = 1.0;

    fn to_f32(self) -> f32 {
        self
    }

    fn from_f32(value: f32) -> Self {
        value
    }
}

/// A gray pixel: one sample, from black to white.
pub type Luma<T> = [T; 1];
/// A gray pixel and its alpha (opacity).
pub type LumaA<T> = [T; 2];
/// A color pixel: red, green and blue.
pub type Rgb<T> = [T; 3];
/// A color pixel and its alpha (opacity).
pub type Rgba<T> = [T; 4];

/// A pixel type: a fixed number of samples of one `Sample` type.
///
/// Every pixel converts to and from straight-alpha RGBA in `f32`,
/// which is how `Image::convert` turns any pixel type into any other.
pub trait Pixel: Copy + Default + PartialEq + Debug + Send + Sync {
    /// The type of each sample.
    type Sample: Sample;

    /// How many samples a pixel has.
    const CHANNELS: usize;

    /// Converts to `[r, g, b, a]`. Each sample mapped with `Sample::to_f32`.
    /// Pixels without alpha are opaque (alpha 1.0).
    fn to_rgba(self) -> [f32; 4];

    /// Converts from `[r, g, b, a]`. Gray Pixels get the luma of the color,
    /// and pixels without alpha drop it.
    fn from_rgba(rgba: [f32; 4]) -> Self;
}

/// The BT.709 weights of red, green and blue in luma.
const LUMA_WEIGHTS: [f32; 3] = [0.2126, 0.7152, 0.0722];

/// How bright a color looks, as one gray value.
fn luma(r: f32, g: f32, b: f32) -> f32 {
    LUMA_WEIGHTS[0] * r + LUMA_WEIGHTS[1] * g + LUMA_WEIGHTS[2] * b
}

impl<T: Sample> Pixel for Luma<T> {
    // For every type `T` that is a `Sample`, `Luma<T>` is a `Pixel`.
    type Sample = T;
    const CHANNELS: usize = 1;

    fn to_rgba(self) -> [f32; 4] {
        let [y] = self;
        let y = y.to_f32();
        [y, y, y, 1.0]
    }

    fn from_rgba([r, g, b, _]: [f32; 4]) -> Self {
        [T::from_f32(luma(r, g, b))]
    }
}

impl<T: Sample> Pixel for LumaA<T> {
    type Sample = T;
    const CHANNELS: usize = 2;

    fn to_rgba(self) -> [f32; 4] {
        let [y, a] = self;
        let y = y.to_f32();
        [y, y, y, a.to_f32()]
    }

    fn from_rgba([r, g, b, a]: [f32; 4]) -> Self {
        [T::from_f32(luma(r, g, b)), T::from_f32(a)]
    }
}

impl<T: Sample> Pixel for Rgb<T> {
    type Sample = T;
    const CHANNELS: usize = 3;

    fn to_rgba(self) -> [f32; 4] {
        let [r, g, b] = self;
        [r.to_f32(), g.to_f32(), b.to_f32(), 1.0]
    }

    fn from_rgba([r, g, b, _]: [f32; 4]) -> Self {
        [T::from_f32(r), T::from_f32(g), T::from_f32(b)]
    }
}

impl<T: Sample> Pixel for Rgba<T> {
    type Sample = T;
    const CHANNELS: usize = 4;

    fn to_rgba(self) -> [f32; 4] {
        let [r, g, b, a] = self;
        [r.to_f32(), g.to_f32(), b.to_f32(), a.to_f32()]
    }

    fn from_rgba([r, g, b, a]: [f32; 4]) -> Self {
        [
            T::from_f32(r),
            T::from_f32(g),
            T::from_f32(b),
            T::from_f32(a),
        ]
    }
}

#[cfg(test)]
mod sample_tests {
    use super::*;

    #[test]
    fn every_u8_survives_f32() {
        for v in 0..=255u8 {
            assert_eq!(u8::from_f32(v.to_f32()), v)
        }
        assert_eq!(255u8.to_f32(), 1.0);
    }

    #[test]
    fn every_u16_survives_f32() {
        for v in 0..=65535u16 {
            assert_eq!(u16::from_f32(v.to_f32()), v);
        }
    }

    #[test]
    fn u8_to_u16_multiplies_by_257() {
        for v in 0..=255u8 {
            assert_eq!(u16::from_f32(v.to_f32()), u16::from(v) * 257);
        }
    }

    #[test]
    fn u16_to_u8_rounds_like_before() {
        // Chapter 2 scaled 16-bit samples to 8 bits with exact integer
        // arithmetic: round(255 * v / 65535), halves up.
        for v in 0..=65535u16 {
            let exact = (2 * 255 * u32::from(v) + 65535) / (2 * 65535);
            assert_eq!(u32::from(u8::from_f32(v.to_f32())), exact, "v = {v}");
        }
    }

    #[test]
    fn integers_clamp_and_floats_do_not() {
        assert_eq!(u8::from_f32(-0.5), 0);
        assert_eq!(u8::from_f32(1.5), 255);
        assert_eq!(u8::from_f32(f32::NAN), 0);
        assert_eq!(u16::from_f32(2.0), 65535);
    }
}

#[cfg(test)]
mod pixel_tests {
    use super::*;

    #[test]
    fn channel_counts() {
        assert_eq!(<Luma<u8> as Pixel>::CHANNELS, 1);
        assert_eq!(<LumaA<u16> as Pixel>::CHANNELS, 2);
        assert_eq!(<Rgb<f32> as Pixel>::CHANNELS, 3);
        assert_eq!(<Rgba<u8> as Pixel>::CHANNELS, 4);
    }

    #[test]
    fn gray_to_color_repeats_the_sample() {
        let gray: Luma<u8> = [51];
        assert_eq!(gray.to_rgba(), [0.2, 0.2, 0.2, 1.0]);
        let rgb: Rgb<u8> = Rgb::from_rgba(gray.to_rgba());
        assert_eq!(rgb, [51, 51, 51]);
    }

    #[test]
    fn color_to_gray_uses_luma_weights() {
        // Pure green is much brighter than pure blue.
        let green: Luma<u8> = Luma::from_rgba([0.0, 1.0, 0.0, 1.0]);
        let blue: Luma<u8> = Luma::from_rgba([0.0, 0.0, 1.0, 1.0]);
        assert_eq!(green, [182]); // 0.7152 * 255 = 182.4
        assert_eq!(blue, [18]); // 0.0722 * 255 = 18.4
        let white: Luma<u16> = Luma::from_rgba([1.0, 1.0, 1.0, 1.0]);
        assert_eq!(white, [65535]);
    }

    #[test]
    fn every_gray_survives_a_trip_through_rgb() {
        for v in 0..=65535u16 {
            let rgb: Rgb<u16> = Rgb::from_rgba([v].to_rgba());
            let back: Luma<u16> = Luma::from_rgba(rgb.to_rgba());
            assert_eq!(back, [v]);
        }
    }

    #[test]
    fn alpha_is_added_opaque_and_dropped() {
        let rgba: Rgba<u8> = Rgba::from_rgba([10u8, 20, 30].to_rgba());
        assert_eq!(rgba, [10, 20, 30, 255]);
        let rgb: Rgb<u8> = Rgb::from_rgba([10u8, 20, 30, 0].to_rgba());
        assert_eq!(rgb, [10, 20, 30]); // the color under the transparent pixel
        let la: LumaA<u16> = LumaA::from_rgba([1.0, 1.0, 1.0, 0.5]);
        assert_eq!(la, [65535, 32768]);
    }
}
