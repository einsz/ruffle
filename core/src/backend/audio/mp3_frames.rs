//! Counting the samples of MPEG audio data from its frame headers.

/// Bit rates in kbit/s by bit rate index, for MPEG-1 layers I, II and III.
const MPEG1_BIT_RATES: [[u16; 15]; 3] = [
    [
        0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
    ],
    [
        0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
    ],
    [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
    ],
];

/// Bit rates in kbit/s by bit rate index, for MPEG-2 and MPEG-2.5 layers I, II and III.
const MPEG2_BIT_RATES: [[u16; 15]; 3] = [
    [
        0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
    ],
    [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
    [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
];

/// The parts of an MPEG audio frame header needed to count samples.
struct FrameHeader {
    /// The length of the frame in bytes, including the header.
    size: usize,

    /// The number of sample frames in the frame.
    sample_frames: u16,

    /// The sample rate of the frame.
    sample_rate: u32,
}

impl FrameHeader {
    /// Parses the first three bytes of an MPEG audio frame header.
    ///
    /// The fourth byte only holds the channel mode, copyright and emphasis.
    fn parse(bytes: &[u8]) -> Option<Self> {
        let &[b0, b1, b2] = bytes else {
            return None;
        };
        if b0 != 0xff || b1 & 0xe0 != 0xe0 {
            return None;
        }
        let version = (b1 >> 3) & 0b11;
        let layer = (b1 >> 1) & 0b11;
        let bit_rate_index = usize::from(b2 >> 4);
        let sample_rate_index = usize::from((b2 >> 2) & 0b11);
        let padding = usize::from((b2 >> 1) & 1);
        // Version 0b01 and layer 0b00 are reserved, bit rate index 0 is the free format,
        // and bit rate index 15 and sample rate index 3 are invalid.
        if version == 0b01
            || layer == 0b00
            || bit_rate_index == 0
            || bit_rate_index == 15
            || sample_rate_index == 3
        {
            return None;
        }

        let is_mpeg1 = version == 0b11;
        // Version 0b10 is MPEG-2 (half the MPEG-1 sample rates), 0b00 MPEG-2.5 (a quarter).
        let sample_rate = [44100, 48000, 32000][sample_rate_index]
            >> match version {
                0b11 => 0,
                0b10 => 1,
                _ => 2,
            };
        // Layer 0b11 is layer I, 0b10 layer II, and 0b01 layer III.
        let layer_index = usize::from(3 - layer);
        let bit_rates = if is_mpeg1 {
            &MPEG1_BIT_RATES
        } else {
            &MPEG2_BIT_RATES
        };
        let bit_rate = usize::from(bit_rates[layer_index][bit_rate_index]) * 1000;
        let sample_rate_usize = sample_rate as usize;

        let (size, sample_frames) = match layer_index {
            // Layer I counts in slots of 4 bytes.
            0 => ((12 * bit_rate / sample_rate_usize + padding) * 4, 384),
            1 => (144 * bit_rate / sample_rate_usize + padding, 1152),
            _ if is_mpeg1 => (144 * bit_rate / sample_rate_usize + padding, 1152),
            _ => (72 * bit_rate / sample_rate_usize + padding, 576),
        };

        Some(Self {
            size,
            sample_frames,
            sample_rate,
        })
    }
}

/// Returns the number of sample frames at `sample_rate` in the given MPEG audio data,
/// as Flash Player counts them for the length of an embedded MP3 sound.
///
/// Every frame header found counts, whatever its MPEG version and layer, with its samples
/// converted from the frame's own sample rate. A byte that doesn't start a valid header is
/// skipped, and a frame counts as soon as the first three bytes of its header are there,
/// even when the data ends inside it. Encoder delay, padding and Xing/Info frames are all
/// included.
pub fn mp3_sample_frames(data: &[u8], sample_rate: u16) -> f64 {
    let sample_rate = f64::from(sample_rate);
    let mut sample_frames = 0.0;
    let mut pos = 0;
    while let Some(header) = data.get(pos..pos + 3) {
        if let Some(frame) = FrameHeader::parse(header) {
            sample_frames +=
                f64::from(frame.sample_frames) * sample_rate / f64::from(frame.sample_rate);
            pos += frame.size;
        } else {
            pos += 1;
        }
    }
    sample_frames
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MPEG-2 layer III, 32 kbit/s, 22050 Hz, mono: 104 bytes without padding.
    const MPEG2_HEADER: [u8; 4] = [0xff, 0xf3, 0x40, 0xc4];

    fn frames(count: usize) -> Vec<u8> {
        let mut data = Vec::new();
        for _ in 0..count {
            data.extend(MPEG2_HEADER);
            data.resize(data.len() + 100, 0);
        }
        data
    }

    #[test]
    fn counts_every_frame() {
        assert_eq!(mp3_sample_frames(&frames(3), 22050), 3.0 * 576.0);
    }

    #[test]
    fn counts_a_truncated_last_frame() {
        let mut data = frames(3);
        data.extend(&MPEG2_HEADER[..3]);
        assert_eq!(mp3_sample_frames(&data, 22050), 4.0 * 576.0);
        data.pop();
        assert_eq!(mp3_sample_frames(&data, 22050), 3.0 * 576.0);
    }

    #[test]
    fn skips_bytes_between_frames() {
        let mut data = vec![0x12, 0xff, 0x00];
        data.extend(frames(1));
        data.extend([0; 37]);
        data.extend(frames(1));
        assert_eq!(mp3_sample_frames(&data, 22050), 2.0 * 576.0);
    }

    #[test]
    fn converts_from_the_frame_sample_rate() {
        // MPEG-1 layer III, 32 kbit/s, 44100 Hz: 1152 sample frames.
        let mut data = vec![0xff, 0xfb, 0x10, 0xc4];
        data.resize(104, 0);
        assert_eq!(mp3_sample_frames(&data, 22050), 576.0);
    }

    #[test]
    fn ignores_invalid_headers() {
        // Reserved version, reserved layer, free format, bad bit rate and bad sample rate.
        for header in [
            [0xff, 0xeb, 0x40],
            [0xff, 0xf1, 0x40],
            [0xff, 0xf3, 0x00],
            [0xff, 0xf3, 0xf0],
            [0xff, 0xf3, 0x4c],
        ] {
            assert_eq!(mp3_sample_frames(&header, 22050), 0.0, "{header:x?}");
        }
    }
}
