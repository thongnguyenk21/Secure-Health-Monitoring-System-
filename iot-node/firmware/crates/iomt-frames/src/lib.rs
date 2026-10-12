//! Compact sample record exchanged between the wearable and the gateway.
//!
//! The crate is `no_std` and allocation-free so it runs on the ESP32-C3 and,
//! unchanged, in host tests. One record is exactly [`RECORD_LEN`] bytes:
//!
//! ```text
//! offset size field
//!   0     4   seq        (u32, little endian) sequence number, +1 per sample
//!   4     4   mono_ms    (u32) device monotonic clock since boot
//!   8     2   hr_bpm     (u16) 0 means "not available"
//!  10     1   spo2_pct   (u8)  0 means "not available"
//!  11     2   steps      (u16) steps counted in this second
//!  13     1   activity   (u8)  see [`Activity`]
//!  14     1   quality    (u8)  0..=100
//!  15     1   crc8       over bytes 0..15 (CRC-8/SMBUS)
//! ```
#![cfg_attr(not(test), no_std)]

/// Size of one encoded [`Record`] in bytes.
pub const RECORD_LEN: usize = 16;

/// Errors produced while decoding a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// The input slice is not exactly [`RECORD_LEN`] bytes long.
    WrongLength(usize),
    /// The checksum does not match the payload (corrupted in transit).
    BadCrc,
    /// The activity byte is not a known [`Activity`] code.
    BadActivity(u8),
}

/// Movement class reported by the wearable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Activity {
    /// Little or no movement.
    Rest = 0,
    /// Walking-level movement.
    Walk = 1,
    /// Running-level movement.
    Run = 2,
    /// The classifier could not decide.
    Unknown = 3,
}

impl TryFrom<u8> for Activity {
    type Error = FrameError;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(Self::Rest),
            1 => Ok(Self::Walk),
            2 => Ok(Self::Run),
            3 => Ok(Self::Unknown),
            other => Err(FrameError::BadActivity(other)),
        }
    }
}

/// One once-per-second measurement produced by the wearable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    /// Sequence number, increases by one for every sample (used for acks).
    pub seq: u32,
    /// Milliseconds since the device booted (monotonic, never jumps).
    pub mono_ms: u32,
    /// Heart rate in beats per minute; `0` means not available.
    pub hr_bpm: u16,
    /// Blood oxygen saturation in percent; `0` means not available.
    pub spo2_pct: u8,
    /// Steps counted during this sample's second.
    pub steps: u16,
    /// Movement class.
    pub activity: Activity,
    /// Signal quality, 0 (useless) to 100 (clean).
    pub quality: u8,
}

impl Record {
    /// An all-zero record, used to pre-fill fixed-size buffers.
    pub const EMPTY: Self = Self {
        seq: 0,
        mono_ms: 0,
        hr_bpm: 0,
        spo2_pct: 0,
        steps: 0,
        activity: Activity::Unknown,
        quality: 0,
    };

    /// Encodes the record into its fixed wire format.
    ///
    /// # Returns
    /// A [`RECORD_LEN`]-byte array whose last byte is the CRC-8 of the first 15.
    #[must_use]
    pub fn encode(&self) -> [u8; RECORD_LEN] {
        let mut out = [0u8; RECORD_LEN];
        out[0..4].copy_from_slice(&self.seq.to_le_bytes());
        out[4..8].copy_from_slice(&self.mono_ms.to_le_bytes());
        out[8..10].copy_from_slice(&self.hr_bpm.to_le_bytes());
        out[10] = self.spo2_pct;
        out[11..13].copy_from_slice(&self.steps.to_le_bytes());
        out[13] = self.activity as u8;
        out[14] = self.quality;
        out[15] = crc8(&out[..15]);
        out
    }

    /// Decodes a record and verifies its checksum.
    ///
    /// # Arguments
    /// * `bytes` - exactly [`RECORD_LEN`] bytes received from the link.
    ///
    /// # Errors
    /// * [`FrameError::WrongLength`] if `bytes.len() != RECORD_LEN`.
    /// * [`FrameError::BadCrc`] if the checksum does not match.
    /// * [`FrameError::BadActivity`] if the activity code is unknown.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let raw: &[u8; RECORD_LEN] = bytes
            .try_into()
            .map_err(|_| FrameError::WrongLength(bytes.len()))?;
        if crc8(&raw[..15]) != raw[15] {
            return Err(FrameError::BadCrc);
        }
        Ok(Self {
            seq: u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]),
            mono_ms: u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]),
            hr_bpm: u16::from_le_bytes([raw[8], raw[9]]),
            spo2_pct: raw[10],
            steps: u16::from_le_bytes([raw[11], raw[12]]),
            activity: Activity::try_from(raw[13])?,
            quality: raw[14],
        })
    }
}

/// CRC-8/SMBUS (polynomial 0x07, initial value 0, no reflection).
///
/// # Arguments
/// * `data` - the bytes to protect.
///
/// # Returns
/// The 8-bit checksum.
#[must_use]
pub fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Record {
        Record {
            seq: 70_000,
            mono_ms: 123_456,
            hr_bpm: 72,
            spo2_pct: 98,
            steps: 2,
            activity: Activity::Walk,
            quality: 90,
        }
    }

    #[test]
    fn crc8_matches_the_standard_check_value() {
        // Published check value for CRC-8/SMBUS over the ASCII string "123456789".
        assert_eq!(crc8(b"123456789"), 0xF4);
    }

    #[test]
    fn encode_decode_round_trip() {
        let bytes = sample().encode();
        assert_eq!(bytes.len(), RECORD_LEN);
        assert_eq!(Record::decode(&bytes), Ok(sample()));
    }

    #[test]
    fn a_flipped_bit_is_detected() {
        let mut bytes = sample().encode();
        bytes[8] ^= 0x01;
        assert_eq!(Record::decode(&bytes), Err(FrameError::BadCrc));
    }

    #[test]
    fn wrong_length_is_rejected() {
        assert_eq!(Record::decode(&[0u8; 15]), Err(FrameError::WrongLength(15)));
    }

    #[test]
    fn unknown_activity_is_rejected() {
        let mut bytes = sample().encode();
        bytes[13] = 9;
        bytes[15] = crc8(&bytes[..15]); // keep the checksum valid
        assert_eq!(Record::decode(&bytes), Err(FrameError::BadActivity(9)));
    }
}
