//! Minimal gateway that receives simulated wearables, for development only.
//!
//! It does what edge-ingest must do at the link level, and nothing more: run the
//! handshake (time anchor and protocol negotiation), check each record's CRC,
//! turn device monotonic time into UTC, drop duplicates, validate the values as
//! domain `SummaryVitals` and acknowledge what arrived. It does not window, infer
//! or upload; that is edge-ingest's job (MVP-04).

use std::collections::HashMap;
use std::io;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use iomt_frames::{Activity as FrameActivity, Record};
use iomt_protocol::firmware_v1 as pb;
use iomt_protocol::link::{self, ProtoRange};
use iomt_protocol::{Activity, DeviceId, DomainError, SignalQuality, SummaryVitals, UnixMs};
use sha2::{Digest, Sha256};

use crate::physio::Rng;
use crate::wire::{self, Frame};

/// Re-run the handshake this often to bound clock drift (REQ-REL-004).
const REHANDSHAKE_EVERY: Duration = Duration::from_secs(600);
/// Acknowledge at least this often while records keep arriving.
const ACK_EVERY: Duration = Duration::from_secs(5);

/// Gateway settings.
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// Address to listen on, e.g. `127.0.0.1:7000`.
    pub listen: String,
    /// Link protocol versions this gateway speaks.
    pub proto: ProtoRange,
    /// Acknowledge after this many new records.
    pub ack_every_records: u32,
}

/// Accepts wearables forever, one thread per connection.
///
/// # Errors
/// Returns an error if the listen address cannot be bound.
pub fn run(cfg: &GatewayConfig) -> io::Result<()> {
    let listener = TcpListener::bind(&cfg.listen)?;
    let seen: SeenSeqs = Arc::default();
    println!(
        "gateway listening on {} (protocol {}..={})",
        cfg.listen, cfg.proto.min, cfg.proto.max
    );
    for conn in listener.incoming() {
        let stream = conn?;
        let cfg = cfg.clone();
        let seen = Arc::clone(&seen);
        thread::spawn(move || match handle(stream, &cfg, &seen) {
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                println!("a device disconnected");
            }
            Err(e) => println!("connection closed: {e}"),
            Ok(()) => {}
        });
    }
    Ok(())
}

/// Pseudonymous id of a device: the first 16 bytes of SHA-256 over its address.
///
/// The address itself never leaves the gateway.
///
/// # Errors
/// Never fails in practice; the hex string always has the required format.
pub fn device_id_for(mac: [u8; 6]) -> Result<DeviceId, DomainError> {
    let digest = Sha256::digest(mac);
    let hex: String = digest.iter().take(16).map(|b| format!("{b:02x}")).collect();
    DeviceId::new(hex)
}

/// Turns a received record into validated domain vitals.
///
/// # Arguments
/// * `device_id` - id of the sending device.
/// * `rec` - decoded record; `0` heart rate or SpO2 means "not available".
/// * `offset_ms` - UTC milliseconds at device monotonic time zero.
///
/// # Errors
/// Returns [`DomainError`] if a value breaks a domain rule.
pub fn to_vitals(
    device_id: &DeviceId,
    rec: &Record,
    offset_ms: i64,
) -> Result<SummaryVitals, DomainError> {
    let vitals = SummaryVitals {
        device_id: device_id.clone(),
        ts: UnixMs(offset_ms + i64::from(rec.mono_ms)),
        hr_bpm: (rec.hr_bpm != 0).then_some(rec.hr_bpm),
        spo2_pct: (rec.spo2_pct != 0).then_some(rec.spo2_pct),
        activity: match rec.activity {
            FrameActivity::Rest => Activity::Rest,
            FrameActivity::Walk => Activity::Walk,
            FrameActivity::Run => Activity::Run,
            FrameActivity::Unknown => Activity::Unknown,
        },
        quality: SignalQuality::new(rec.quality)?,
    };
    vitals.validate()?;
    Ok(vitals)
}

/// A handshake in flight: what the gateway sent and when.
struct PendingHello {
    session_id: u32,
    t0_unix_ms: i64,
    sent_at: Instant,
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Highest sequence number received per device id, kept across reconnections:
/// a device resends what was not acknowledged before the link dropped, and those
/// records must count as duplicates, not as new data.
type SeenSeqs = Arc<Mutex<HashMap<String, u32>>>;

fn handle(mut stream: TcpStream, cfg: &GatewayConfig, seen: &SeenSeqs) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let mut reader = stream.try_clone()?;
    let Frame::PeerAddress(mac) = wire::read_frame(&mut reader)? else {
        return Err(io::Error::other("the first frame must be the peer address"));
    };
    let device_id = device_id_for(mac).map_err(io::Error::other)?;
    let tag = device_id.as_str()[..8].to_owned();
    println!("[{tag}] connected from {}", stream.peer_addr()?);

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        loop {
            let frame = wire::read_frame(&mut reader);
            let ended = frame.is_err();
            if tx.send(frame).is_err() || ended {
                break;
            }
        }
    });

    let mut rng = Rng::new(
        now_unix_ms().unsigned_abs()
            ^ u64::from_le_bytes([mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], 0, 0]),
    );
    let mut pending: Option<PendingHello> = None;
    let mut session_id = 0u32;
    let mut offset_ms: Option<i64> = None;
    let mut last_hello: Option<Instant> = None;
    let mut highest_seq = seen
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(device_id.as_str())
        .copied()
        .unwrap_or(0);
    let mut unacked = 0u32;
    let mut last_ack = Instant::now();
    let mut duplicates = 0u32;

    loop {
        if pending.is_none() && last_hello.is_none_or(|t| t.elapsed() >= REHANDSHAKE_EVERY) {
            let hello = PendingHello {
                session_id: rng.range(1.0, f64::from(u32::MAX)) as u32,
                t0_unix_ms: now_unix_ms(),
                sent_at: Instant::now(),
            };
            send(
                &mut stream,
                pb::link_control::Msg::Hello(pb::LinkHello {
                    session_id: hello.session_id,
                    t0_unix_ms: hello.t0_unix_ms,
                }),
            )?;
            pending = Some(hello);
            last_hello = Some(Instant::now());
        }

        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(frame) => match frame? {
                Frame::LinkControl(bytes) => {
                    if let pb::link_control::Msg::Report(report) = wire::decode_link(&bytes)? {
                        let Some(hello) = pending.take_if(|h| h.session_id == report.session_id)
                        else {
                            continue;
                        };
                        let node = ProtoRange {
                            min: report.proto_min,
                            max: report.proto_max,
                        };
                        let outcome = link::negotiate(cfg.proto, node);
                        send(
                            &mut stream,
                            pb::link_control::Msg::Select(link::select_message(
                                hello.session_id,
                                outcome,
                            )),
                        )?;
                        if let Err(refusal) = outcome {
                            println!(
                                "[{tag}] ALARM: link refused ({refusal}); data from this device is ignored"
                            );
                            return Ok(());
                        }
                        let rtt_ms =
                            i64::try_from(hello.sent_at.elapsed().as_millis()).unwrap_or(0);
                        let mono0_ms = i64::try_from(report.mono0_us / 1000).unwrap_or(i64::MAX);
                        offset_ms = Some(hello.t0_unix_ms + rtt_ms / 2 - mono0_ms);
                        session_id = hello.session_id;
                        println!(
                            "[{tag}] handshake ok: fw {} board {} protocol {}..={} battery {}% rtt {rtt_ms} ms",
                            report.fw_version,
                            report.board,
                            report.proto_min,
                            report.proto_max,
                            report.battery_pct.map_or("?".to_owned(), |b| b.to_string())
                        );
                    }
                }
                Frame::Record(bytes) => {
                    let Some(offset) = offset_ms else {
                        continue;
                    };
                    let rec = match Record::decode(&bytes) {
                        Ok(rec) => rec,
                        Err(e) => {
                            println!("[{tag}] dropped a corrupted record: {e:?}");
                            continue;
                        }
                    };
                    if rec.seq <= highest_seq {
                        duplicates += 1;
                        continue;
                    }
                    if rec.seq > highest_seq + 1 && highest_seq != 0 {
                        println!(
                            "[{tag}] gap: seq {} -> {} (lost on the device)",
                            highest_seq, rec.seq
                        );
                    }
                    highest_seq = rec.seq;
                    seen.lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .insert(device_id.as_str().to_owned(), highest_seq);
                    unacked += 1;
                    match to_vitals(&device_id, &rec, offset) {
                        Ok(v) => println!("[{tag}] {}", describe(&v, rec.seq)),
                        Err(e) => println!("[{tag}] rejected seq {}: {e}", rec.seq),
                    }
                }
                Frame::PeerAddress(_) => {}
            },
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::other("reader stopped"));
            }
        }

        if unacked > 0 && (unacked >= cfg.ack_every_records || last_ack.elapsed() >= ACK_EVERY) {
            send(
                &mut stream,
                pb::link_control::Msg::Ack(pb::LinkAck {
                    session_id,
                    through_seq: highest_seq,
                }),
            )?;
            if duplicates > 0 {
                println!("[{tag}] ack through seq {highest_seq} ({duplicates} duplicates ignored)");
                duplicates = 0;
            }
            unacked = 0;
            last_ack = Instant::now();
        }
    }
}

fn send(stream: &mut TcpStream, msg: pb::link_control::Msg) -> io::Result<()> {
    wire::write_frame(stream, &wire::link_frame(msg))
}

/// One readable line per sample, time of day in UTC.
fn describe(v: &SummaryVitals, seq: u32) -> String {
    let ms_of_day = v.ts.0.rem_euclid(86_400_000);
    let (h, m, s, ms) = (
        ms_of_day / 3_600_000,
        ms_of_day / 60_000 % 60,
        ms_of_day / 1000 % 60,
        ms_of_day % 1000,
    );
    let hr = v.hr_bpm.map_or("  -".to_owned(), |x| format!("{x:3}"));
    let spo2 = v.spo2_pct.map_or("  -".to_owned(), |x| format!("{x:3}"));
    let flag = if v.hr_bpm.is_some_and(|x| x > 140) {
        "  HIGH HR"
    } else {
        ""
    };
    format!(
        "seq {seq:5} {h:02}:{m:02}:{s:02}.{ms:03}Z  HR {hr} bpm  SpO2 {spo2} %  {:<7} q {:3}{flag}",
        format!("{:?}", v.activity).to_lowercase(),
        v.quality.value()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_device_id_is_a_stable_hash_of_the_address() {
        let a = device_id_for([2, 0, 0x5e, 0x10, 0, 1]).unwrap();
        let b = device_id_for([2, 0, 0x5e, 0x10, 0, 1]).unwrap();
        let c = device_id_for([2, 0, 0x5e, 0x10, 0, 2]).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(
            !a.as_str().contains("025e10"),
            "the address must not be readable"
        );
    }

    #[test]
    fn records_become_utc_vitals() {
        let id = device_id_for([2, 0, 0x5e, 0x10, 0, 1]).unwrap();
        let rec = Record {
            seq: 5,
            mono_ms: 4_000,
            hr_bpm: 72,
            spo2_pct: 0,
            steps: 1,
            activity: FrameActivity::Walk,
            quality: 88,
        };
        let v = to_vitals(&id, &rec, 1_760_000_000_000).unwrap();
        assert_eq!(v.ts, UnixMs(1_760_000_004_000));
        assert_eq!((v.hr_bpm, v.spo2_pct), (Some(72), None));
        assert_eq!(v.activity, Activity::Walk);
    }

    #[test]
    fn implausible_records_are_rejected() {
        let id = device_id_for([2, 0, 0x5e, 0x10, 0, 1]).unwrap();
        let rec = Record {
            seq: 1,
            mono_ms: 0,
            hr_bpm: 400,
            spo2_pct: 98,
            steps: 0,
            activity: FrameActivity::Rest,
            quality: 90,
        };
        assert_eq!(
            to_vitals(&id, &rec, 0),
            Err(DomainError::HeartRateOutOfRange(400))
        );
    }
}
