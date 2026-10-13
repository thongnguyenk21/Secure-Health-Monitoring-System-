//! One simulated wearable: the loop the firmware will run, over TCP instead of BLE.
//!
//! Every second the loop takes a reading and records it into the offline ring
//! first, connected or not. While the link is up and the handshake has selected
//! a protocol, every record not yet sent goes out oldest-first; records leave
//! the ring only on `LinkAck`. Walking out of range drops the link, the ring
//! fills up, and whatever is still buffered is resent after the reconnection.

use std::io;
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use iomt_frames::Record;
use iomt_protocol::firmware_v1 as pb;
use iomt_protocol::link::ProtoRange;
use offline_buffer::{Courier, Sample};

use crate::physio::{Body, HighHrEpisode, Rng};
use crate::wire::{self, Frame};

/// Ring capacity: 900 samples at 1 Hz keep 15 minutes offline (decision D21).
pub const RING_CAPACITY: usize = 900;
/// Seconds to wait after the gateway refused the link before trying again.
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
    /// Position in the fleet, 0-based; gives the address and the seed.
    pub index: u32,
    /// Gateway address, e.g. `127.0.0.1:7000`.
    pub gateway: String,
    /// Base seed; the wearer uses `seed + index`.
    pub seed: u64,
    /// Link protocol versions this firmware speaks.
    pub proto: ProtoRange,
    /// Optional out-of-range window.
    pub out_of_range: Option<OutOfRange>,
    /// Optional high heart rate episode.
    pub high_hr: Option<HighHrEpisode>,
    /// Stop after this many seconds; `None` runs forever.
    pub duration_s: Option<u32>,
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

/// What a finished run leaves in the ring, for tests and the final log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunReport {
    /// Samples recorded.
    pub recorded: u32,
    /// Samples still waiting for an acknowledgement.
    pub pending: usize,
    /// Samples lost to ring overflow.
    pub dropped: u32,
}

/// Facts about this boot that the handshake reports.
struct Firmware {
    boot: Instant,
    boot_id: u32,
    proto: ProtoRange,
    label: String,
}

/// Live connection to the gateway.
struct Link {
    stream: TcpStream,
    rx: Receiver<io::Result<Frame>>,
    session: Option<u32>,
    streaming: bool,
    last_sent: u32,
}

impl Drop for Link {
    fn drop(&mut self) {
        // The reader thread holds a clone of the socket, so dropping this
        // handle alone would leave the connection open.
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

impl Link {
    fn connect(addr: SocketAddr, mac: [u8; 6]) -> io::Result<Self> {
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))?;
        stream.set_nodelay(true)?;
        wire::write_frame(&mut stream, &Frame::PeerAddress(mac))?;
        let rx = wire::spawn_reader(stream.try_clone()?);
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
    /// Replying at once matters: the gateway assumes the `LinkReport` left half
    /// a round trip after its `LinkHello`, so a reply held back until the next
    /// sample would skew every timestamp by half the delay.
    ///
    /// # Errors
    /// The link failed, or the gateway refused it (`ConnectionRefused`).
    fn pump(
        &mut self,
        courier: &mut Courier<RING_CAPACITY>,
        fw: &Firmware,
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
                        mono0_us: u64::try_from(fw.boot.elapsed().as_micros()).unwrap_or(u64::MAX),
                        fw_version: concat!(env!("CARGO_PKG_VERSION"), "-sim").to_owned(),
                        board: "sim-wearable".to_owned(),
                        proto_min: fw.proto.min,
                        proto_max: fw.proto.max,
                        battery_pct: Some(battery_pct),
                        boot_id: fw.boot_id,
                    }))?;
                }
                pb::link_control::Msg::Select(select)
                    if Some(select.session_id) == self.session =>
                {
                    match select.outcome {
                        Some(pb::link_select::Outcome::ProtoSelected(version)) => {
                            if !self.streaming {
                                println!("[{}] link up, protocol v{version}", fw.label);
                            }
                            self.streaming = true;
                        }
                        Some(pb::link_select::Outcome::Refusal(code)) => {
                            let reason = pb::LinkRefusal::try_from(code)
                                .map_or("unknown", |r| r.as_str_name());
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
    ///
    /// `last_sent` starts at 0 on each new connection, so a reconnection resends
    /// everything still in the ring.
    fn send_pending(&mut self, courier: &Courier<RING_CAPACITY>) -> io::Result<()> {
        let mut batch = [Record::EMPTY; RING_CAPACITY];
        let n = courier.peek_batch(&mut batch);
        let already_sent = self.last_sent;
        for rec in batch[..n].iter().filter(|r| r.seq > already_sent) {
            wire::write_frame(&mut self.stream, &Frame::Record(rec.encode()))?;
            self.last_sent = rec.seq;
        }
        Ok(())
    }
}

/// A random, non-zero id for this boot, different on every run.
fn new_boot_id(index: u32) -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let mut rng = Rng::new(u64::from(nanos) ^ (u64::from(index) << 32));
    (rng.unit() * f64::from(u32::MAX)) as u32 | 1
}

/// Runs one wearable, one sample per second.
///
/// # Arguments
/// * `cfg` - identity, gateway address and scenario of this wearable.
///
/// # Returns
/// The state of the ring when `cfg.duration_s` ran out.
///
/// # Errors
/// Only when the gateway address cannot be resolved; link failures are logged
/// and retried, as a real device would.
pub fn run(cfg: &DeviceConfig) -> io::Result<RunReport> {
    let addr = cfg
        .gateway
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::other(format!("cannot resolve {}", cfg.gateway)))?;
    let fw = Firmware {
        boot: Instant::now(),
        boot_id: new_boot_id(cfg.index),
        proto: cfg.proto,
        label: cfg.label(),
    };
    let label = &fw.label;
    println!("[{label}] booted, boot id {:08x}", fw.boot_id);

    let mut courier: Courier<RING_CAPACITY> = Courier::new();
    let mut body = Body::new(cfg.seed.wrapping_add(u64::from(cfg.index)), cfg.high_hr);
    let mut link: Option<Link> = None;
    let mut retry_at_s = 0u32;
    let mut was_out_of_range = false;
    let mut next_tick = Instant::now();
    let mut recorded = 0u32;

    for t_s in 0u32.. {
        if cfg.duration_s.is_some_and(|d| t_s >= d) {
            break;
        }

        // 1. Measure and record first, connected or not: the ring keeps it.
        let r = body.step(t_s);
        courier.record(Sample {
            mono_ms: fw.boot.elapsed().as_millis() as u32,
            hr_bpm: r.hr_bpm,
            spo2_pct: r.spo2_pct,
            steps: r.steps,
            activity: r.activity,
            quality: r.quality,
        });
        recorded += 1;
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

        // 3. Send what is not sent yet, then listen to the gateway until the
        //    next sample is due.
        next_tick += Duration::from_secs(1);
        if let Some(l) = link.as_mut() {
            let result = if l.streaming {
                l.send_pending(&courier)
            } else {
                Ok(())
            }
            .and_then(|()| l.pump(&mut courier, &fw, battery_pct, next_tick));
            if let Err(e) = result {
                println!("[{label}] link down: {e}");
                if e.kind() == io::ErrorKind::ConnectionRefused {
                    retry_at_s = t_s + REFUSAL_BACKOFF_S;
                }
                link = None;
            }
        }

        if t_s % 30 == 29 {
            let online = link.as_ref().is_some_and(|l| l.streaming);
            println!(
                "[{label}] t={}s buffered {} lost {} {}",
                t_s + 1,
                courier.pending(),
                courier.dropped(),
                if online { "online" } else { "offline" }
            );
        }
        thread::sleep(next_tick.saturating_duration_since(Instant::now()));
    }

    Ok(RunReport {
        recorded,
        pending: courier.pending(),
        dropped: courier.dropped(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::{self, GatewayConfig};
    use std::net::TcpListener;

    fn start_gateway() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let cfg = GatewayConfig {
            proto: ProtoRange { min: 1, max: 1 },
            ack_every_records: 1,
        };
        thread::spawn(move || gateway::serve(&listener, cfg));
        addr
    }

    fn device(gateway: String, duration_s: u32) -> DeviceConfig {
        DeviceConfig {
            index: 0,
            gateway,
            seed: 42,
            proto: ProtoRange { min: 1, max: 1 },
            out_of_range: None,
            high_hr: None,
            duration_s: Some(duration_s),
        }
    }

    #[test]
    fn addresses_follow_the_index() {
        let mut cfg = device(String::new(), 1);
        cfg.index = 0x0102;
        assert_eq!(cfg.address(), [0x02, 0x00, 0x5e, 0x10, 0x01, 0x02]);
    }

    #[test]
    fn boot_ids_are_never_zero() {
        for index in 0..100 {
            assert_ne!(new_boot_id(index), 0);
        }
    }

    #[test]
    fn a_connected_device_gets_its_samples_acknowledged() {
        let report = run(&device(start_gateway(), 3)).unwrap();
        assert_eq!(report.recorded, 3);
        assert_eq!(report.dropped, 0);
        assert!(
            report.pending <= 1,
            "only the last sample may still wait: {report:?}"
        );
    }

    #[test]
    fn samples_taken_out_of_range_are_resent_after_coming_back() {
        let mut cfg = device(start_gateway(), 6);
        cfg.out_of_range = Some(OutOfRange { at_s: 1, for_s: 3 });
        let report = run(&cfg).unwrap();
        assert_eq!(report.recorded, 6);
        assert_eq!(report.dropped, 0);
        assert!(
            report.pending <= 1,
            "the backlog was acknowledged: {report:?}"
        );
    }

    #[test]
    fn without_a_gateway_everything_stays_in_the_ring() {
        let free = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = free.local_addr().unwrap().to_string();
        drop(free);
        let report = run(&device(addr, 2)).unwrap();
        assert_eq!((report.recorded, report.pending), (2, 2));
    }
}
