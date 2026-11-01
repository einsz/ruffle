use crate::{Fixed16, Rectangle, Twips};
use bitflags::bitflags;

/// How much each pass should multiply the requested blur size by - accumulative.
/// These are very approximate to Flash, and not 100% exact.
/// Pass 1 would be 100%, but pass 2 would be 110%.
/// This is accumulative so you can calculate the size upfront for how many passes you'll need to perform.
const PASS_SCALES: [f64; 15] = [
    1.0, 2.1, 2.7, 3.1, 3.5, 3.8, 4.0, 4.2, 4.4, 4.6, 5.0, 6.0, 6.0, 7.0, 7.0,
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlurFilter {
    pub blur_x: Fixed16,
    pub blur_y: Fixed16,
    pub flags: BlurFilterFlags,
}

impl BlurFilter {
    #[inline]
    pub fn num_passes(&self) -> u8 {
        (self.flags & BlurFilterFlags::PASSES).bits() >> 3
    }

    pub fn scale(&mut self, x: f32, y: f32) {
        self.blur_x = Self::scale_blur(self.blur_x, x);
        self.blur_y = Self::scale_blur(self.blur_y, y);
    }

    pub fn impotent(&self) -> bool {
        self.num_passes() == 0 || (self.blur_x <= Fixed16::ONE && self.blur_y <= Fixed16::ONE)
    }

    pub(crate) fn pass_scale(&self) -> f64 {
        PASS_SCALES[self.num_passes().clamp(1, 15) as usize - 1]
    }

    /// The area that the blur reaches, as in Flash: about half as far as
    /// `calculate_dest_rect`, in whole pixels. Without passes, nothing is
    /// blurred.
    pub fn calculate_area(&self, source_rect: Rectangle<Twips>) -> Rectangle<Twips> {
        if self.num_passes() == 0 {
            return source_rect;
        }
        let scale = self.pass_scale();
        let extent =
            |blur: Fixed16| Twips::from_pixels((scale * blur.to_f64() / 2.0).floor() + 1.0);
        let (x, y) = (extent(self.blur_x), extent(self.blur_y));
        Rectangle {
            x_min: source_rect.x_min - x,
            x_max: source_rect.x_max + x,
            y_min: source_rect.y_min - y,
            y_max: source_rect.y_max + y,
        }
    }

    pub fn calculate_dest_rect(&self, source_rect: Rectangle<Twips>) -> Rectangle<Twips> {
        let scale = self.pass_scale();
        let x = Twips::from_pixels((scale * self.blur_x.to_f64()).max(0.0));
        let y = Twips::from_pixels((scale * self.blur_y.to_f64()).max(0.0));
        Rectangle {
            x_min: source_rect.x_min - x,
            x_max: source_rect.x_max + x,
            y_min: source_rect.y_min - y,
            y_max: source_rect.y_max + y,
        }
    }

    #[inline]
    pub(crate) fn scale_blur(blur: Fixed16, factor: f32) -> Fixed16 {
        (blur - Fixed16::ONE) * Fixed16::from_f32(factor) + Fixed16::ONE
    }
}

/// A filter's offset, rounded down to whole pixels as in Flash's filter areas.
pub(crate) fn pixel_offset(angle: Fixed16, distance: Fixed16) -> (Twips, Twips) {
    let (angle, distance) = (angle.to_f64(), distance.to_f64());
    // The angle is stored in fixed point, so e.g. 90 degrees are slightly
    // less, which mustn't round a whole offset down.
    let floor = |offset: f64| Twips::from_pixels((offset + 1e-3).floor());
    (floor(angle.cos() * distance), floor(angle.sin() * distance))
}

/// The area that a filter with an offset draws to, as in Flash: the blurred
/// source moved by the offset, together with the source.
pub(crate) fn moved_area(
    source_rect: Rectangle<Twips>,
    blurred: Rectangle<Twips>,
    (x, y): (Twips, Twips),
) -> Rectangle<Twips> {
    Rectangle {
        x_min: source_rect.x_min.min(blurred.x_min + x),
        x_max: source_rect.x_max.max(blurred.x_max + x),
        y_min: source_rect.y_min.min(blurred.y_min + y),
        y_max: source_rect.y_max.max(blurred.y_max + y),
    }
}

/// The area that a bevel draws to, as in Flash: the blurred source,
/// extended by the offset on both sides.
pub(crate) fn extended_area(blurred: Rectangle<Twips>, (x, y): (Twips, Twips)) -> Rectangle<Twips> {
    let (x, y) = (Twips::new(x.get().abs()), Twips::new(y.get().abs()));
    Rectangle {
        x_min: blurred.x_min - x,
        x_max: blurred.x_max + x,
        y_min: blurred.y_min - y,
        y_max: blurred.y_max + y,
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct BlurFilterFlags: u8 {
        const PASSES = 0b11111 << 3;
    }
}

impl BlurFilterFlags {
    #[inline]
    pub fn from_passes(num_passes: u8) -> Self {
        let flags = Self::from_bits_retain(num_passes << 3);
        debug_assert_eq!(flags & Self::PASSES, flags);
        flags
    }
}
