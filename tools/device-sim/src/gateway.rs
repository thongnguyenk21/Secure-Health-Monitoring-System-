//! Minimal gateway that receives simulated wearables, for development only.
//!
//! It does what edge-ingest must do at the link level, and nothing more: run the
//! handshake (time anchor, protocol negotiation, boot id), check each record's
//! CRC, turn device monotonic time into UTC, drop duplicates, validate the
//! values as domain `SummaryVitals` and acknowledge what arrived. It does not
//! window, infer or upload; that is edge-ingest's job (MVP-04).

use std::collections::HashMap;
use std::fmt;
use std::io::{self, Read};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use iomt_frames::{Activity as FrameActivity, Record};
use iomt_protocol::firmware_v1 as pb;
use iomt_protocol::link::{self, LinkRefusal, ProtoRange};
use iomt_protocol::{Activity, DeviceId, DomainError, SignalQuality, SummaryVitals, UnixMs};
use sha2::Sha256;

use crate::physio::Rng;
use crate::wire::{self, Frame};

/// Re-run the handshake this often to bound clock drift (REQ-REL-004).
const REHANDSHAKE_EVERY: Duration = Duration::from_secs(600);
/// Acknowledge at least this often while records keep arriving.
const ACK_EVERY: Duration = Duration::from_secs(5);
/// A wearable that ignores this many handshakes in a row is disconnected.
const MAX_HELLO_TRIES: u32 = 3;

/// Secret that turns a device address into a pseudonymous id (HMAC-SHA-256 key).
///
/// A plain hash of a 48-bit address can be reversed by trying every address, so
/// the id is keyed: without this secret nobody can map an id back to an address.
/// A real gateway loads the key from its secret store; this stub generates one
/// at start-up or reads it from the command line.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct IdKey([u8; 32]);

impl fmt::Debug for IdKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IdKey(..)") // never print the secret
    }
}

impl IdKey {
    /// Creates a key from 32 random bytes of the operating system.
    ///
    /// # Errors
    /// Returns the I/O error if `/dev/urandom` cannot be read.
    pub fn random() -> io::Result<Self> {
        let mut key = [0u8; 32];
        std::fs::File::open("/dev/urandom")?.read_exact(&mut key)?;
        Ok(Self(key))
    }

    /// Parses a key written as 64 hexadecimal characters.
    ///
    /// # Arguments
    /// * `hex` - the key, e.g. from a secret store.
    ///
    /// # Errors
    /// Returns a message if `hex` is not exactly 64 hexadecimal characters.
    pub fn from_hex(hex: &str) -> Result<Self, String> {
        if hex.len() != 64 || !hex.is_ascii() {
            return Err(format!(
                "the id key must be 64 hex characters, got {}",
                hex.len()
            ));
        }
        let mut key = [0u8; 32];
        for (byte, pair) in key.iter_mut().zip(hex.as_bytes().chunks(2)) {
            let text = std::str::from_utf8(pair).map_err(|e| e.to_string())?;
            *byte = u8::from_str_radix(text, 16).map_err(|_| format!("{text:?} is not hex"))?;
        }
        Ok(Self(key))
    }
}

/// Gateway settings.
#[derive(Debug, Clone, Copy)]
pub struct GatewayConfig {
    /// Link protocol versions this gateway speaks.
    pub proto: ProtoRange,
    /// Acknowledge after this many new records.
    pub ack_every_records: u32,
    /// How long to wait for a wearable's answer to a handshake before asking again.
    pub hello_timeout: Duration,
    /// Wearables served at the same time; further connections are refused.
    pub max_connections: usize,
    /// Secret behind the pseudonymous device ids.
    pub id_key: IdKey,
}

impl GatewayConfig {
    /// Settings with the defaults for everything but the protocol and the ack rate.
    ///
    /// # Arguments
    /// * `proto` - link protocol versions this gateway speaks.
    /// * `ack_every_records` - acknowledge after this many new records.
    /// * `id_key` - secret behind the device ids.
    #[must_use]
    pub const fn new(proto: ProtoRange, ack_every_records: u32, id_key: IdKey) -> Self {
        Self {
            proto,
            ack_every_records,
            hello_timeout: Duration::from_secs(5),
            max_connections: 64,
            id_key,
        }
    }
}

/// Highest sequence number received from each device: `(boot id, seq)`.
///
/// Shared by all connections and kept across reconnections: a device resends
/// what was not acknowledged before the link dropped, and those records are
/// duplicates. Sequence numbers restart after a reboot, so a different boot id
/// replaces the entry. One entry per device keeps the map from growing with
/// every reboot.
type SeenSeqs = Arc<Mutex<HashMap<String, (u32, u32)>>>;

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
/// A failed accept or an over-limit connection is logged and skipped; it never
/// stops the gateway for the wearables that are already connected.
///
/// # Errors
/// Never returns an error today; the `Result` keeps room for a shutdown signal.
pub fn serve(listener: &TcpListener, cfg: GatewayConfig) -> io::Result<()> {
    let seen: SeenSeqs = Arc::default();
    let active = Arc::new(AtomicUsize::new(0));
    for conn in listener.incoming() {
        let stream = match conn {
            Ok(stream) => stream,
            Err(e) => {
                println!("accept failed: {e}");
                thread::sleep(Duration::from_millis(50)); // do not spin on a persistent error
                continue;
            }
        };
        if active.load(Ordering::Relaxed) >= cfg.max_connections {
            println!("too many wearables connected: refused one");
            let _ = stream.shutdown(Shutdown::Both);
            continue;
        }
        let Ok(closer) = stream.try_clone() else {
            println!("could not clone a connection: refused one");
            continue;
        };
        active.fetch_add(1, Ordering::Relaxed);
        let seen = Arc::clone(&seen);
        let active = Arc::clone(&active);
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
            active.fetch_sub(1, Ordering::Relaxed);
        });
    }
    Ok(())
}

/// Pseudonymous id of a device: the first 16 bytes of HMAC-SHA-256 over its address.
///
/// The address itself never leaves the gateway, and without `key` the id cannot
/// be mapped back to it (a plain hash of a 48-bit address could be).
///
/// # Arguments
/// * `key` - the gateway's secret.
/// * `mac` - the device address.
///
/// # Errors
/// Never fails in practice; the key length is always valid and the hex string
/// always has the required format.
pub fn device_id_for(key: &IdKey, mac: [u8; 6]) -> Result<DeviceId, DomainError> {
    let mut mac_tag = Hmac::<Sha256>::new_from_slice(&key.0)
        .map_err(|_| DomainError::InvalidDeviceId { len: 0 })?; // unreachable: HMAC takes any key length
    mac_tag.update(&mac);
    let digest = mac_tag.finalize().into_bytes();
    let hex: String = digest.iter().take(16).map(|b| format!("{b:02x}")).collect();
    DeviceId::new(hex)
}

/// Pairs one instant on the device clock with the same instant in UTC.
///
/// The device clock is a `u32` of milliseconds that wraps every 49.7 days, so a
/// record's time is computed from its distance to the anchor in wrapping
/// arithmetic. Records older than the anchor (an offline backlog) come out
/// earlier, records newer come out later; this is right as long as a record is
/// less than 24.8 days away from the anchor, which the 10-minute re-handshake
/// guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeAnchor {
    unix_ms: i64,
    mono_ms: u32,
}

impl TimeAnchor {
    /// Creates an anchor.
    ///
    /// # Arguments
    /// * `unix_ms` - UTC milliseconds at the anchor instant.
    /// * `mono_ms` - device monotonic milliseconds at the same instant (wrapped to `u32`).
    #[must_use]
    pub const fn new(unix_ms: i64, mono_ms: u32) -> Self {
        Self { unix_ms, mono_ms }
    }

    /// Converts a device timestamp to UTC milliseconds.
    ///
    /// # Arguments
    /// * `mono_ms` - a record's device monotonic time.
    ///
    /// # Returns
    /// UTC milliseconds since the Unix epoch.
    #[must_use]
    pub fn to_unix_ms(self, mono_ms: u32) -> i64 {
        let distance = mono_ms.wrapping_sub(self.mono_ms);
        // Reinterpret the wrapped distance as signed: negative means "before the anchor".
        self.unix_ms + i64::from(i32::from_ne_bytes(distance.to_ne_bytes()))
    }
}

/// Turns a received record into validated domain vitals.
///
/// # Arguments
/// * `device_id` - id of the sending device.
/// * `rec` - decoded record; `0` heart rate or SpO2 means "not available".
/// * `anchor` - maps the device clock to UTC.
///
/// # Errors
/// Returns [`DomainError`] if a value breaks a domain rule.
pub fn to_vitals(
    device_id: &DeviceId,
    rec: &Record,
    anchor: TimeAnchor,
) -> Result<SummaryVitals, DomainError> {
    let vitals = SummaryVitals {
        device_id: device_id.clone(),
        ts: UnixMs(anchor.to_unix_ms(rec.mono_ms)),
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
    anchor: TimeAnchor,
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

fn send(stream: &mut TcpStream, msg: pb::link_control::Msg) -> io::Result<()> {
    wire::write_frame(stream, &wire::link_frame(msg))
}

/// Highest sequence number already received from `device` during `boot_id`, or 0.
fn load_seq(seen: &SeenSeqs, device: &str, boot_id: u32) -> u32 {
    seen.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(device)
        .filter(|(boot, _)| *boot == boot_id)
        .map_or(0, |(_, seq)| *seq)
}

/// Remembers the highest sequence number of `device`; a new boot replaces the old entry.
fn store_seq(seen: &SeenSeqs, device: &str, boot_id: u32, seq: u32) {
    seen.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(device.to_owned(), (boot_id, seq));
}

/// Serves one connected wearable until it disconnects or is refused.
fn handle(mut stream: TcpStream, cfg: GatewayConfig, seen: &SeenSeqs) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let mut reader = stream.try_clone()?;
    let Frame::PeerAddress(mac) = wire::read_frame(&mut reader)? else {
        return Err(io::Error::other("the first frame must be the peer address"));
    };
    let device_id = device_id_for(&cfg.id_key, mac).map_err(io::Error::other)?;
    let tag = device_id.as_str()[..8].to_owned();
    println!("[{tag}] connected from {}", stream.peer_addr()?);
    let rx = wire::spawn_reader(reader);

    let mut rng =
        Rng::new(now_unix_ms().unsigned_abs() ^ u64::from(mac[5]) << 8 ^ u64::from(mac[4]));
    let mut pending: Option<PendingHello> = None;
    let mut session: Option<Session> = None;
    let mut last_hello: Option<Instant> = None;
    let mut hello_tries = 0u32;
    let mut highest_seq = 0u32;
    let mut unacked = 0u32;
    let mut duplicates = 0u32;
    let mut last_ack = Instant::now();

    loop {
        // 1. Give up on a handshake the wearable never answers: ask again, and
        //    disconnect a wearable that stays silent. Then start a handshake on
        //    connect and every 10 minutes.
        if pending
            .as_ref()
            .is_some_and(|h| h.sent_at.elapsed() >= cfg.hello_timeout)
        {
            pending = None;
            if session.is_none() {
                hello_tries += 1;
                if hello_tries >= MAX_HELLO_TRIES {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "the wearable never answered the handshake",
                    ));
                }
                last_hello = None; // ask again right away
            }
        }
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
                    // Records carry the device clock as a wrapping u32, so the anchor
                    // keeps only the low 32 bits of the reported microsecond clock.
                    let mono0_ms =
                        u32::try_from((report.mono0_us / 1000) % (1 << 32)).unwrap_or_default();
                    if session.as_ref().is_none_or(|s| s.boot_id != report.boot_id) {
                        highest_seq = load_seq(seen, device_id.as_str(), report.boot_id);
                    }
                    hello_tries = 0;
                    session = Some(Session {
                        session_id: hello.session_id,
                        boot_id: report.boot_id,
                        anchor: TimeAnchor::new(hello.t0_unix_ms + rtt_ms / 2, mono0_ms),
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
                    let Some((boot_id, anchor)) = session.as_ref().map(|s| (s.boot_id, s.anchor))
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
                        // A resend of something already received still needs an
                        // acknowledgement, or the wearable never frees its ring.
                        duplicates += 1;
                        unacked += 1;
                        continue;
                    }
                    if highest_seq != 0 && rec.seq > highest_seq + 1 {
                        println!(
                            "[{tag}] gap: seq {highest_seq} -> {} (lost on the device)",
                            rec.seq
                        );
                    }
                    highest_seq = rec.seq;
                    store_seq(seen, device_id.as_str(), boot_id, highest_seq);
                    unacked += 1;
                    match to_vitals(&device_id, &rec, anchor) {
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

    fn test_key() -> IdKey {
        IdKey::from_hex(&"ab".repeat(32)).unwrap()
    }

    fn config(ack_every_records: u32) -> GatewayConfig {
        GatewayConfig::new(ProtoRange { min: 1, max: 1 }, ack_every_records, test_key())
    }

    fn start_with(cfg: GatewayConfig) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || serve(&listener, cfg));
        addr
    }

    fn start_gateway(ack_every_records: u32) -> SocketAddr {
        start_with(config(ack_every_records))
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
    fn the_device_id_is_stable_for_one_key_and_address() {
        let key = test_key();
        let a = device_id_for(&key, MAC).unwrap();
        assert_eq!(a, device_id_for(&key, MAC).unwrap());
        assert_ne!(a, device_id_for(&key, [2, 0, 0x5e, 0x10, 0, 2]).unwrap());
        assert!(
            !a.as_str().contains("025e10"),
            "the address must not be readable"
        );
    }

    #[test]
    fn the_device_id_cannot_be_recomputed_without_the_key() {
        use sha2::Digest;
        let id = device_id_for(&test_key(), MAC).unwrap();
        let other_key = IdKey::from_hex(&"cd".repeat(32)).unwrap();
        assert_ne!(
            id,
            device_id_for(&other_key, MAC).unwrap(),
            "another secret, another id"
        );
        let plain: String = sha2::Sha256::digest(MAC)
            .iter()
            .take(16)
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_ne!(
            id.as_str(),
            plain,
            "a plain hash of the address would be reversible"
        );
    }

    #[test]
    fn the_id_key_parses_hex_and_stays_secret_in_debug_output() {
        assert!(IdKey::from_hex(&"0f".repeat(32)).is_ok());
        assert!(IdKey::from_hex("abcd").is_err(), "too short");
        assert!(IdKey::from_hex(&"zz".repeat(32)).is_err(), "not hex");
        assert_eq!(format!("{:?}", test_key()), "IdKey(..)");
    }

    #[test]
    fn device_clock_times_are_continuous_across_the_u32_wrap() {
        // The device clock wraps 500 ms after the anchor; the next record is 2 s later.
        let anchor = TimeAnchor::new(1_000_000, u32::MAX - 500);
        assert_eq!(anchor.to_unix_ms(u32::MAX - 500), 1_000_000);
        assert_eq!(anchor.to_unix_ms(1_500), 1_000_000 + 2_001);
    }

    #[test]
    fn records_older_than_the_anchor_come_out_earlier() {
        let anchor = TimeAnchor::new(1_000_000, 10_000);
        assert_eq!(anchor.to_unix_ms(4_000), 1_000_000 - 6_000);
        // Also when the older record was taken just before the clock wrapped.
        let wrapped = TimeAnchor::new(1_000_000, 100);
        assert_eq!(wrapped.to_unix_ms(u32::MAX - 99), 1_000_000 - 200);
    }

    #[test]
    fn only_the_latest_boot_of_a_device_is_remembered() {
        let seen: SeenSeqs = Arc::default();
        store_seq(&seen, "dev", 1, 10);
        assert_eq!(load_seq(&seen, "dev", 1), 10);
        assert_eq!(load_seq(&seen, "dev", 2), 0, "a new boot starts at zero");
        store_seq(&seen, "dev", 2, 3);
        assert_eq!(load_seq(&seen, "dev", 1), 0, "the old boot is forgotten");
        assert_eq!(seen.lock().unwrap().len(), 1, "one entry per device");
    }

    #[test]
    fn records_become_utc_vitals() {
        let id = device_id_for(&test_key(), MAC).unwrap();
        let rec = Record {
            seq: 5,
            mono_ms: 4_000,
            hr_bpm: 72,
            spo2_pct: 0,
            steps: 1,
            activity: FrameActivity::Walk,
            quality: 88,
        };
        let v = to_vitals(&id, &rec, TimeAnchor::new(1_760_000_000_000, 0)).unwrap();
        assert_eq!(v.ts, UnixMs(1_760_000_004_000));
        assert_eq!((v.hr_bpm, v.spo2_pct), (Some(72), None));
    }

    #[test]
    fn implausible_records_are_rejected() {
        let id = device_id_for(&test_key(), MAC).unwrap();
        let mut rec = Record::EMPTY;
        rec.hr_bpm = 400;
        rec.quality = 90;
        assert_eq!(
            to_vitals(&id, &rec, TimeAnchor::new(0, 0)),
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

    #[test]
    fn a_resend_of_known_records_is_acknowledged_again() {
        let addr = start_gateway(3);
        let mut first = FakeDevice::connect(addr);
        first.handshake(7, 1, 1);
        for seq in 1..=3 {
            first.send_record(seq);
        }
        assert_eq!(first.next_ack(), 3);
        first.disconnect(); // pretend the ack never reached the wearable

        let mut again = FakeDevice::connect(addr);
        again.handshake(7, 1, 1); // same boot id: the wearable did not reboot
        for seq in 1..=3 {
            again.send_record(seq); // everything it still holds is a duplicate
        }
        assert_eq!(
            again.next_ack(),
            3,
            "the wearable must be able to free its ring"
        );
    }

    #[test]
    fn a_wearable_that_never_answers_the_handshake_is_disconnected() {
        let cfg = GatewayConfig {
            hello_timeout: Duration::from_millis(100),
            ..config(1)
        };
        let dev = FakeDevice::connect(start_with(cfg));
        let mut hellos = 0;
        loop {
            match dev.rx.recv_timeout(WAIT) {
                Ok(Ok(Frame::LinkControl(bytes))) => {
                    assert!(matches!(
                        wire::decode_link(&bytes),
                        Ok(pb::link_control::Msg::Hello(_))
                    ));
                    hellos += 1;
                }
                Ok(Ok(_)) => {}
                Ok(Err(_)) => break, // the gateway closed the connection
                Err(_) => panic!("the gateway never gave up on a silent wearable"),
            }
        }
        assert_eq!(
            hellos, MAX_HELLO_TRIES,
            "it asks a few times before giving up"
        );
    }

    #[test]
    fn extra_wearables_are_refused_and_a_freed_slot_is_reusable() {
        let cfg = GatewayConfig {
            max_connections: 1,
            ..config(1)
        };
        let addr = start_with(cfg);
        let mut first = FakeDevice::connect(addr);
        first.handshake(1, 1, 1);

        let second = FakeDevice::connect(addr);
        assert!(
            matches!(second.rx.recv_timeout(WAIT), Ok(Err(_))),
            "the second wearable must be turned away without a handshake"
        );

        first.disconnect();
        let deadline = Instant::now() + WAIT;
        loop {
            let third = FakeDevice::connect(addr);
            if let Ok(Ok(Frame::LinkControl(_))) = third.rx.recv_timeout(Duration::from_millis(300))
            {
                break; // got a LinkHello: the slot was freed
            }
            assert!(Instant::now() < deadline, "the slot was never freed");
        }
    }
}
