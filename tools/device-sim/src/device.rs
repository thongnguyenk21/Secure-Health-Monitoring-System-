//! One simulated wearable, behaving like the firmware over the TCP link.
//!
//! Every second it records a sample into the offline ring buffer first, exactly
//! like the firmware. While the link is up and the handshake has selected a
//! protocol, every record not yet sent goes out oldest-first; records leave the
//! ring only when the gateway acknowledges them. Walking out of range drops the
//! connection, the ring fills up, and everything still buffered is resent after
//! the reconnection.

use std::io;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use iomt_frames::Record;
use iomt_protocol::firmware_v1 as pb;
use iomt_protocol::link::ProtoRange;
use offline_buffer::Courier;

use crate::physio::{Body, HighHrEpisode};
use crate::wire::{self, Frame};

/// Ring capacity: 900 samples at 1 Hz keep 15 minutes offline (design D21).
pub const RING_CAPACITY: usize = 900;
/// Seconds to wait after the gateway refused the protocol before retrying.
const REFUSAL_BACKOFF_S: u32 = 30;

/// Window during which the wearer is out of the gateway's range.
#[derive(Debug, Clone, Copy)]
pub struct OutOfRange {
    /// Start, seconds since the simulation started.
    pub at_s: u32,
    /// Duration in seconds.
    pub for_s: u32,
}

/// Everything one simulated wearable needs.
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    /// Position in the fleet, 0-based; also gives the address and the seed.
    pub index: u32,
    /// Gateway address, e.g. `127.0.0.1:7000`.
    pub gateway: String,
    /// Base seed; the device uses `seed + index`.
    pub seed: u64,
    /// Link protocol versions this firmware speaks.
    pub proto: ProtoRange,
    /// Optional out-of-range window.
    pub out_of_range: Option<OutOfRange>,
    /// Optional high heart rate episode.
    pub high_hr: Option<HighHrEpisode>,
}

impl DeviceConfig {
    /// Locally administered address derived from the index, like a BLE MAC.
    #[must_use]
    pub fn address(&self) -> [u8; 6] {
        let [.., hi, lo] = self.index.to_be_bytes();
        [0x02, 0x00, 0x5e, 0x10, hi, lo]
    }

    fn label(&self) -> String {
        format!("sim-{:02}", self.index)
    }
}

/// Live connection to the gateway.
struct Link {
    stream: TcpStream,
    rx: Receiver<io::Result<Frame>>,
    session: Option<u32>,
    streaming: bool,
    last_sent: u32,
}

impl Link {
    fn connect(addr: SocketAddr, mac: [u8; 6]) -> io::Result<Self> {
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))?;
        stream.set_nodelay(true)?;
        wire::write_frame(&mut stream, &Frame::PeerAddress(mac))?;
        let mut reader = stream.try_clone()?;
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
        Ok(Self {
            stream,
            rx,
            session: None,
            streaming: false,
            last_sent: 0,
        })
    }

    fn send(&mut self, msg: pb::link_control::Msg) -> io::Result<()> {
        wire::write_frame(&mut self.stream, &wire::link_frame(msg))
    }

    /// Answers the gateway as soon as it speaks, until `until`.
    ///
    /// Replying at once matters: the gateway assumes the LinkReport left half a
    /// round trip after its LinkHello, so a reply held back until the next sample
    /// would skew every timestamp by half the delay.
    fn pump(
        &mut self,
        courier: &mut Courier<RING_CAPACITY>,
        boot: Instant,
        cfg: &DeviceConfig,
        battery_pct: u32,
        until: Instant,
    ) -> io::Result<()> {
        loop {
            let wait = until.saturating_duration_since(Instant::now());
            let frame = match self.rx.recv_timeout(wait) {
                Ok(frame) => frame?,
                Err(RecvTimeoutError::Timeout) => return Ok(()),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(io::Error::other("link reader stopped"));
                }
            };
            let Frame::LinkControl(bytes) = frame else {
                continue;
            };
            match wire::decode_link(&bytes)? {
                pb::link_control::Msg::Hello(hello) => {
                    self.session = Some(hello.session_id);
                    self.send(pb::link_control::Msg::Report(pb::LinkReport {
                        session_id: hello.session_id,
                        mono0_us: u64::try_from(boot.elapsed().as_micros()).unwrap_or(u64::MAX),
                        fw_version: concat!(env!("CARGO_PKG_VERSION"), "-sim").to_owned(),
                        board: "sim-wearable".to_owned(),
                        proto_min: cfg.proto.min,
                        proto_max: cfg.proto.max,
                        battery_pct: Some(battery_pct),
                    }))?;
                }
                pb::link_control::Msg::Select(select)
                    if Some(select.session_id) == self.session =>
                {
                    match select.outcome {
                        Some(pb::link_select::Outcome::ProtoSelected(version)) => {
                            if !self.streaming {
                                println!("[{}] link up, protocol v{version}", cfg.label());
                            }
                            self.streaming = true;
                        }
                        Some(pb::link_select::Outcome::Refusal(code)) => {
                            let reason = pb::LinkRefusal::try_from(code)
                                .map_or("unknown".to_owned(), |r| r.as_str_name().to_owned());
                            return Err(io::Error::new(
                                io::ErrorKind::ConnectionRefused,
                                format!("gateway refused the link: {reason}"),
                            ));
                        }
                        None => return Err(io::Error::other("LinkSelect without an outcome")),
                    }
                }
                pb::link_control::Msg::Ack(ack) => {
                    courier.on_ack(ack.through_seq);
                }
                pb::link_control::Msg::Select(_) | pb::link_control::Msg::Report(_) => {}
            }
        }
    }

    /// Sends every buffered record newer than the last one sent, oldest first.
    fn send_pending(&mut self, courier: &Courier<RING_CAPACITY>) -> io::Result<()> {
        let mut batch = vec![Record::EMPTY; courier.pending()];
        let n = courier.peek_batch(&mut batch);
        let already_sent = self.last_sent;
        for rec in batch.iter().take(n).filter(|r| r.seq > already_sent) {
            wire::write_frame(&mut self.stream, &Frame::Record(rec.encode()))?;
            self.last_sent = rec.seq;
        }
        Ok(())
    }
}

/// Runs one wearable forever, one sample per second.
///
/// # Arguments
/// * `cfg` - identity, gateway address and scenario of this wearable.
///
/// # Errors
/// Returns an error only when the gateway address cannot be resolved; link
/// failures are logged and retried, as a real device would.
pub fn run(cfg: &DeviceConfig) -> io::Result<()> {
    let addr = cfg
        .gateway
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::other(format!("cannot resolve {}", cfg.gateway)))?;
    let label = cfg.label();
    let boot = Instant::now();
    let mut courier: Courier<RING_CAPACITY> = Courier::new();
    let mut body = Body::new(cfg.seed.wrapping_add(u64::from(cfg.index)), cfg.high_hr);
    let mut link: Option<Link> = None;
    let mut retry_at_s = 0u32;
    let mut was_out_of_range = false;
    let mut next_tick = Instant::now();

    for t_s in 0u32.. {
        // 1. Always record first, connected or not (the ring keeps it).
        let mono_ms = boot.elapsed().as_millis() as u32;
        let s = body.step(t_s);
        courier.record_sample(
            mono_ms, s.hr_bpm, s.spo2_pct, s.steps, s.activity, s.quality,
        );
        let battery_pct = 100u32.saturating_sub(t_s / 360);

        // 2. Range: leaving it drops the link like a BLE disconnection.
        let out_of_range = cfg
            .out_of_range
            .is_some_and(|w| t_s >= w.at_s && t_s < w.at_s.saturating_add(w.for_s));
        if out_of_range != was_out_of_range {
            println!(
                "[{label}] {} (buffered {}, lost {})",
                if out_of_range {
                    "walked out of range"
                } else {
                    "back in range"
                },
                courier.pending(),
                courier.dropped()
            );
            was_out_of_range = out_of_range;
        }
        if out_of_range {
            link = None;
        } else if link.is_none() && t_s >= retry_at_s {
            match Link::connect(addr, cfg.address()) {
                Ok(l) => link = Some(l),
                Err(e) if t_s % 10 == 0 => println!("[{label}] cannot reach gateway: {e}"),
                Err(_) => {}
            }
        }

        // 3. Stream what is not sent yet, then listen to the gateway until the
        //    next sample is due.
        next_tick += Duration::from_secs(1);
        if let Some(l) = link.as_mut() {
            let result = if l.streaming {
                l.send_pending(&courier)
            } else {
                Ok(())
            }
            .and_then(|()| l.pump(&mut courier, boot, cfg, battery_pct, next_tick));
            if let Err(e) = result {
                println!("[{label}] link down: {e}");
                if e.kind() == io::ErrorKind::ConnectionRefused {
                    retry_at_s = t_s + REFUSAL_BACKOFF_S;
                }
                link = None;
            }
        }

        if t_s % 30 == 29 {
            println!(
                "[{label}] t={}s buffered {} lost {} {}",
                t_s + 1,
                courier.pending(),
                courier.dropped(),
                if link.as_ref().is_some_and(|l| l.streaming) {
                    "online"
                } else {
                    "offline"
                }
            );
        }

        thread::sleep(next_tick.saturating_duration_since(Instant::now()));
    }
    Ok(())
}
