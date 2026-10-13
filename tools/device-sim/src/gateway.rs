//! Minimal gateway that receives simulated wearables, for development only.
//!
//! It does what edge-ingest must do at the link level, and nothing more: run the
//! handshake (time anchor, protocol negotiation, boot id), check each record's
//! CRC, turn device monotonic time into UTC, drop duplicates, validate the
//! values as domain `SummaryVitals` and acknowledge what arrived. It does not
//! window, infer or upload; that is edge-ingest's job (MVP-04).

use std::collections::HashMap;
use std::io;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use iomt_frames::{Activity as FrameActivity, Record};
use iomt_protocol::firmware_v1 as pb;
use iomt_protocol::link::{self, LinkRefusal, ProtoRange};
use iomt_protocol::{Activity, DeviceId, DomainError, SignalQuality, SummaryVitals, UnixMs};
use sha2::{Digest, Sha256};

use crate::physio::Rng;
use crate::wire::{self, Frame};

/// Re-run the handshake this often to bound clock drift (REQ-REL-004).
const REHANDSHAKE_EVERY: Duration = Duration::from_secs(600);
/// Acknowledge at least this often while records keep arriving.
const ACK_EVERY: Duration = Duration::from_secs(5);

/// Gateway settings.
#[derive(Debug, Clone, Copy)]
pub struct GatewayConfig {
    /// Link protocol versions this gateway speaks.
    pub proto: ProtoRange,
    /// Acknowledge after this many new records.
    pub ack_every_records: u32,
}

/// Highest sequence number received per (device id, boot id), shared by all
/// connections. Kept across reconnections: a device resends what was not
/// acknowledged before the link dropped, and those records are duplicates. A
/// new boot id starts at zero, because sequence numbers restart after a reboot.
type SeenSeqs = Arc<Mutex<HashMap<(String, u32), u32>>>;

/// Binds the address and serves wearables forever.
///
/// # Errors
/// Returns an error if the address cannot be bound.
pub fn run(listen: &str, cfg: GatewayConfig) -> io::Result<()> {
    let listener = TcpListener::bind(listen)?;
    println!(
        "gateway listening on {listen} (protocol {}..={})",
        cfg.proto.min, cfg.proto.max
    );
    serve(&listener, cfg)
}

/// Accepts wearables forever, one thread per connection.
///
/// # Errors
/// Returns an error if accepting a connection fails.
pub fn serve(listener: &TcpListener, cfg: GatewayConfig) -> io::Result<()> {
    let seen: SeenSeqs = Arc::default();
    for conn in listener.incoming() {
        let stream = conn?;
        let closer = stream.try_clone()?;
        let seen = Arc::clone(&seen);
        thread::spawn(move || {
            match handle(stream, cfg, &seen) {
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    println!("a device disconnected");
                }
                Err(e) => println!("connection closed: {e}"),
                Ok(()) => {}
            }
            // The reader thread holds a clone of the socket; shutting it down
            // is what tells the device the link is gone.
            let _ = closer.shutdown(Shutdown::Both);
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

/// Decides from a `LinkReport` whether the wearable may stream.
///
/// # Returns
/// The protocol version to use.
///
/// # Errors
/// [`LinkRefusal::MissingBootId`] when `boot_id` is 0, otherwise the result of
/// [`link::negotiate`].
pub fn check_report(gateway: ProtoRange, report: &pb::LinkReport) -> Result<u32, LinkRefusal> {
    if report.boot_id == 0 {
        return Err(LinkRefusal::MissingBootId);
    }
    let node = ProtoRange {
        min: report.proto_min,
        max: report.proto_max,
    };
    link::negotiate(gateway, node)
}

/// A handshake in flight: what the gateway sent and when.
struct PendingHello {
    session_id: u32,
    t0_unix_ms: i64,
    sent_at: Instant,
}

/// The agreed link with the current boot of a device.
struct Session {
    session_id: u32,
    boot_id: u32,
    offset_ms: i64,
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

fn send(stream: &mut TcpStream, msg: pb::link_control::Msg) -> io::Result<()> {
    wire::write_frame(stream, &wire::link_frame(msg))
}

fn load_seq(seen: &SeenSeqs, key: &(String, u32)) -> u32 {
    seen.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(key)
        .copied()
        .unwrap_or(0)
}

fn store_seq(seen: &SeenSeqs, key: (String, u32), seq: u32) {
    seen.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, seq);
}

/// Serves one connected wearable until it disconnects or is refused.
fn handle(mut stream: TcpStream, cfg: GatewayConfig, seen: &SeenSeqs) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let mut reader = stream.try_clone()?;
    let Frame::PeerAddress(mac) = wire::read_frame(&mut reader)? else {
        return Err(io::Error::other("the first frame must be the peer address"));
    };
    let device_id = device_id_for(mac).map_err(io::Error::other)?;
    let tag = device_id.as_str()[..8].to_owned();
    println!("[{tag}] connected from {}", stream.peer_addr()?);
    let rx = wire::spawn_reader(reader);

    let mut rng =
        Rng::new(now_unix_ms().unsigned_abs() ^ u64::from(mac[5]) << 8 ^ u64::from(mac[4]));
    let mut pending: Option<PendingHello> = None;
    let mut session: Option<Session> = None;
    let mut last_hello: Option<Instant> = None;
    let mut highest_seq = 0u32;
    let mut unacked = 0u32;
    let mut duplicates = 0u32;
    let mut last_ack = Instant::now();

    loop {
        // 1. Start a handshake on connect and every 10 minutes.
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

        // 2. Handle whatever the wearable sent.
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(frame) => match frame? {
                Frame::LinkControl(bytes) => {
                    let pb::link_control::Msg::Report(report) = wire::decode_link(&bytes)? else {
                        continue;
                    };
                    let Some(hello) = pending.take_if(|h| h.session_id == report.session_id) else {
                        continue;
                    };
                    let outcome = check_report(cfg.proto, &report);
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
                    let rtt_ms = i64::try_from(hello.sent_at.elapsed().as_millis()).unwrap_or(0);
                    let mono0_ms = i64::try_from(report.mono0_us / 1000).unwrap_or(i64::MAX);
                    if session.as_ref().is_none_or(|s| s.boot_id != report.boot_id) {
                        highest_seq =
                            load_seq(seen, &(device_id.as_str().to_owned(), report.boot_id));
                    }
                    session = Some(Session {
                        session_id: hello.session_id,
                        boot_id: report.boot_id,
                        offset_ms: hello.t0_unix_ms + rtt_ms / 2 - mono0_ms,
                    });
                    println!(
                        "[{tag}] handshake ok: boot {:08x} fw {} protocol {}..={} battery {}% rtt {rtt_ms} ms",
                        report.boot_id,
                        report.fw_version,
                        report.proto_min,
                        report.proto_max,
                        report.battery_pct.map_or("?".to_owned(), |b| b.to_string())
                    );
                }
                Frame::Record(bytes) => {
                    let Some((boot_id, offset_ms)) =
                        session.as_ref().map(|s| (s.boot_id, s.offset_ms))
                    else {
                        continue; // no record is trusted before the handshake
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
                    if highest_seq != 0 && rec.seq > highest_seq + 1 {
                        println!(
                            "[{tag}] gap: seq {highest_seq} -> {} (lost on the device)",
                            rec.seq
                        );
                    }
                    highest_seq = rec.seq;
                    store_seq(seen, (device_id.as_str().to_owned(), boot_id), highest_seq);
                    unacked += 1;
                    match to_vitals(&device_id, &rec, offset_ms) {
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

        // 3. Acknowledge every N new records, or every 5 s while some wait.
        if let Some(s) = session.as_ref()
            && unacked > 0
            && (unacked >= cfg.ack_every_records || last_ack.elapsed() >= ACK_EVERY)
        {
            send(
                &mut stream,
                pb::link_control::Msg::Ack(pb::LinkAck {
                    session_id: s.session_id,
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
    use std::net::SocketAddr;
    use std::sync::mpsc::Receiver;

    const MAC: [u8; 6] = [2, 0, 0x5e, 0x10, 0, 1];
    const WAIT: Duration = Duration::from_secs(5);

    fn start_gateway(ack_every_records: u32) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let cfg = GatewayConfig {
            proto: ProtoRange { min: 1, max: 1 },
            ack_every_records,
        };
        thread::spawn(move || serve(&listener, cfg));
        addr
    }

    /// The device side of the link, driven by hand inside a test.
    struct FakeDevice {
        stream: TcpStream,
        rx: Receiver<io::Result<Frame>>,
    }

    impl FakeDevice {
        fn connect(addr: SocketAddr) -> Self {
            let mut stream = TcpStream::connect(addr).unwrap();
            wire::write_frame(&mut stream, &Frame::PeerAddress(MAC)).unwrap();
            let rx = wire::spawn_reader(stream.try_clone().unwrap());
            Self { stream, rx }
        }

        fn next_link(&self) -> pb::link_control::Msg {
            loop {
                if let Frame::LinkControl(bytes) = self.rx.recv_timeout(WAIT).unwrap().unwrap() {
                    return wire::decode_link(&bytes).unwrap();
                }
            }
        }

        fn handshake(&mut self, boot_id: u32, proto_min: u32, proto_max: u32) -> pb::LinkSelect {
            let pb::link_control::Msg::Hello(hello) = self.next_link() else {
                panic!("expected LinkHello first");
            };
            let report = pb::LinkReport {
                session_id: hello.session_id,
                mono0_us: 0,
                fw_version: "test".into(),
                board: "test".into(),
                proto_min,
                proto_max,
                battery_pct: None,
                boot_id,
            };
            send(&mut self.stream, pb::link_control::Msg::Report(report)).unwrap();
            let pb::link_control::Msg::Select(select) = self.next_link() else {
                panic!("expected LinkSelect");
            };
            select
        }

        fn send_record(&mut self, seq: u32) {
            let rec = Record {
                seq,
                mono_ms: seq * 1000,
                hr_bpm: 72,
                spo2_pct: 98,
                steps: 1,
                activity: FrameActivity::Walk,
                quality: 90,
            };
            wire::write_frame(&mut self.stream, &Frame::Record(rec.encode())).unwrap();
        }

        fn next_ack(&self) -> u32 {
            loop {
                if let pb::link_control::Msg::Ack(ack) = self.next_link() {
                    return ack.through_seq;
                }
            }
        }

        fn disconnect(self) {
            let _ = self.stream.shutdown(Shutdown::Both);
        }
    }

    fn selected(select: &pb::LinkSelect) -> Option<u32> {
        match select.outcome {
            Some(pb::link_select::Outcome::ProtoSelected(v)) => Some(v),
            _ => None,
        }
    }

    fn refusal(select: &pb::LinkSelect) -> Option<pb::LinkRefusal> {
        match select.outcome {
            Some(pb::link_select::Outcome::Refusal(code)) => pb::LinkRefusal::try_from(code).ok(),
            _ => None,
        }
    }

    #[test]
    fn the_device_id_is_a_stable_hash_of_the_address() {
        let a = device_id_for(MAC).unwrap();
        assert_eq!(a, device_id_for(MAC).unwrap());
        assert_ne!(a, device_id_for([2, 0, 0x5e, 0x10, 0, 2]).unwrap());
        assert!(
            !a.as_str().contains("025e10"),
            "the address must not be readable"
        );
    }

    #[test]
    fn records_become_utc_vitals() {
        let id = device_id_for(MAC).unwrap();
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
    }

    #[test]
    fn implausible_records_are_rejected() {
        let id = device_id_for(MAC).unwrap();
        let mut rec = Record::EMPTY;
        rec.hr_bpm = 400;
        rec.quality = 90;
        assert_eq!(
            to_vitals(&id, &rec, 0),
            Err(DomainError::HeartRateOutOfRange(400))
        );
    }

    #[test]
    fn a_device_is_acknowledged_after_the_handshake() {
        let mut dev = FakeDevice::connect(start_gateway(3));
        assert_eq!(selected(&dev.handshake(5, 1, 1)), Some(1));
        for seq in 1..=3 {
            dev.send_record(seq);
        }
        assert_eq!(dev.next_ack(), 3);
    }

    #[test]
    fn a_missing_boot_id_is_refused() {
        let mut dev = FakeDevice::connect(start_gateway(1));
        let select = dev.handshake(0, 1, 1);
        assert_eq!(refusal(&select), Some(pb::LinkRefusal::MissingBootId));
    }

    #[test]
    fn a_newer_protocol_is_refused() {
        let mut dev = FakeDevice::connect(start_gateway(1));
        let select = dev.handshake(5, 2, 3);
        assert_eq!(refusal(&select), Some(pb::LinkRefusal::NodeTooNew));
    }

    #[test]
    fn records_resent_after_a_reconnection_are_duplicates() {
        let addr = start_gateway(1);
        let mut dev = FakeDevice::connect(addr);
        dev.handshake(5, 1, 1);
        dev.send_record(1);
        dev.send_record(2);
        assert_eq!(dev.next_ack(), 1);
        assert_eq!(dev.next_ack(), 2);
        dev.disconnect();

        let mut dev = FakeDevice::connect(addr);
        dev.handshake(5, 1, 1);
        for seq in 1..=3 {
            dev.send_record(seq);
        }
        assert_eq!(
            dev.next_ack(),
            3,
            "1 and 2 were duplicates, so no ack for them"
        );
    }

    #[test]
    fn a_reboot_starts_a_fresh_sequence() {
        let addr = start_gateway(1);
        let mut dev = FakeDevice::connect(addr);
        dev.handshake(5, 1, 1);
        for seq in 1..=3 {
            dev.send_record(seq);
        }
        assert_eq!(dev.next_ack(), 1);
        dev.disconnect();

        let mut dev = FakeDevice::connect(addr);
        dev.handshake(6, 1, 1);
        dev.send_record(1);
        assert_eq!(dev.next_ack(), 1, "seq 1 of a new boot is new data");
    }
}
