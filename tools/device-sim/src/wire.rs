//! TCP framing that stands in for the BLE link.
//!
//! On a real wearable the gateway learns the peer address from BLE, reads
//! `link_control` writes and receives 16-byte records as GATT notifications. Over
//! TCP each of those becomes one frame:
//!
//! ```text
//! offset size field
//!   0     1   kind      1 = peer address, 2 = link control, 3 = record
//!   1     2   length    payload length in bytes (u16, little endian), <= 512
//!   3     n   payload   6-byte address | protobuf LinkControl | 16-byte record
//! ```
//!
//! The peer address frame is sent once, first, by the device: it replaces the BLE
//! address the Pi would otherwise see.

use std::io::{self, Read, Write};

use iomt_frames::RECORD_LEN;
use iomt_protocol::firmware_v1 as pb;
use prost::Message;

/// Largest payload accepted, so a corrupted length cannot allocate megabytes.
pub const MAX_PAYLOAD: usize = 512;

const KIND_PEER_ADDRESS: u8 = 1;
const KIND_LINK_CONTROL: u8 = 2;
const KIND_RECORD: u8 = 3;

/// One message on the simulated link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// The device's 6-byte address, standing in for the BLE address.
    PeerAddress([u8; 6]),
    /// A serialized `iomt.firmware.v1.LinkControl`.
    LinkControl(Vec<u8>),
    /// One encoded `iomt_frames::Record`.
    Record([u8; RECORD_LEN]),
}

/// Writes one frame and flushes it.
///
/// # Errors
/// Returns the I/O error of the underlying stream, or `InvalidInput` if the
/// payload is larger than [`MAX_PAYLOAD`].
pub fn write_frame(w: &mut impl Write, frame: &Frame) -> io::Result<()> {
    let (kind, payload): (u8, &[u8]) = match frame {
        Frame::PeerAddress(addr) => (KIND_PEER_ADDRESS, addr),
        Frame::LinkControl(bytes) => (KIND_LINK_CONTROL, bytes),
        Frame::Record(bytes) => (KIND_RECORD, bytes),
    };
    if payload.len() > MAX_PAYLOAD {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("payload of {} bytes exceeds {MAX_PAYLOAD}", payload.len()),
        ));
    }
    let len = u16::try_from(payload.len()).map_err(io::Error::other)?;
    let mut buf = Vec::with_capacity(3 + payload.len());
    buf.push(kind);
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(payload);
    w.write_all(&buf)?;
    w.flush()
}

/// Reads exactly one frame, blocking until it is complete.
///
/// # Errors
/// Returns `UnexpectedEof` when the peer closed the connection, and
/// `InvalidData` for an unknown kind, an oversized length or a payload whose
/// size does not match its kind.
pub fn read_frame(r: &mut impl Read) -> io::Result<Frame> {
    let mut head = [0u8; 3];
    r.read_exact(&mut head)?;
    let len = usize::from(u16::from_le_bytes([head[1], head[2]]));
    if len > MAX_PAYLOAD {
        return Err(invalid(format!("frame length {len} exceeds {MAX_PAYLOAD}")));
    }
    let mut payload = vec![0u8; len];
    r.read_exact(&mut payload)?;
    match head[0] {
        KIND_PEER_ADDRESS => payload
            .try_into()
            .map(Frame::PeerAddress)
            .map_err(|_| invalid("peer address must be 6 bytes")),
        KIND_LINK_CONTROL => Ok(Frame::LinkControl(payload)),
        KIND_RECORD => payload
            .try_into()
            .map(Frame::Record)
            .map_err(|_| invalid(format!("record must be {RECORD_LEN} bytes"))),
        other => Err(invalid(format!("unknown frame kind {other}"))),
    }
}

/// Wraps a link control message into a frame.
#[must_use]
pub fn link_frame(msg: pb::link_control::Msg) -> Frame {
    Frame::LinkControl(pb::LinkControl { msg: Some(msg) }.encode_to_vec())
}

/// Decodes the payload of a link control frame.
///
/// # Errors
/// Returns `InvalidData` if the bytes are not a `LinkControl` or it is empty.
pub fn decode_link(bytes: &[u8]) -> io::Result<pb::link_control::Msg> {
    pb::LinkControl::decode(bytes)
        .map_err(|e| invalid(format!("bad link control message: {e}")))?
        .msg
        .ok_or_else(|| invalid("empty link control message"))
}

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(frame: &Frame) -> Frame {
        let mut buf = Vec::new();
        write_frame(&mut buf, frame).unwrap();
        read_frame(&mut buf.as_slice()).unwrap()
    }

    #[test]
    fn every_kind_survives_a_round_trip() {
        let frames = [
            Frame::PeerAddress([2, 0, 0x5e, 0x10, 0, 1]),
            link_frame(pb::link_control::Msg::Ack(pb::LinkAck {
                session_id: 9,
                through_seq: 120,
            })),
            Frame::Record([7; RECORD_LEN]),
        ];
        for frame in frames {
            assert_eq!(round_trip(&frame), frame);
        }
    }

    #[test]
    fn link_messages_decode_back() {
        let hello = pb::link_control::Msg::Hello(pb::LinkHello {
            session_id: 1,
            t0_unix_ms: 1_760_000_000_000,
        });
        let Frame::LinkControl(bytes) = link_frame(hello.clone()) else {
            panic!("expected a link control frame");
        };
        assert_eq!(decode_link(&bytes).unwrap(), hello);
    }

    #[test]
    fn a_truncated_frame_reports_end_of_stream() {
        let mut buf = Vec::new();
        write_frame(&mut buf, &Frame::Record([1; RECORD_LEN])).unwrap();
        buf.truncate(10);
        let err = read_frame(&mut buf.as_slice()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn malformed_frames_are_rejected() {
        let oversized = [KIND_LINK_CONTROL, 0xff, 0xff];
        let wrong_record_size = [KIND_RECORD, 2, 0, 1, 2];
        let unknown_kind = [9, 0, 0];
        for bytes in [&oversized[..], &wrong_record_size, &unknown_kind] {
            let err = read_frame(&mut &bytes[..]).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }
    }
}
