//! `device-sim`: simulated wearables and a gateway stub, linked over TCP.
//!
//! ```text
//! device-sim gateway                            # terminal 1: receive and print
//! device-sim device                             # terminal 2: one wearable
//! device-sim device --count 3                   # three wearables
//! device-sim device --scenario out-of-range     # leave range for 2 min after 60 s
//! device-sim device --scenario high-hr          # heart rate above 140 bpm
//! device-sim demo                               # vitals and one framed LinkAck
//! ```
//!
//! The devices behave like the firmware (ring buffer, handshake, acks) and send
//! real 16-byte records and `iomt.firmware.v1.LinkControl` messages, so the data
//! has the same shape as from hardware. `wire` describes the TCP framing that
//! replaces BLE.

mod device;
mod gateway;
mod physio;
mod wire;

use std::process::ExitCode;
use std::thread;

use clap::{Parser, Subcommand, ValueEnum};
use iomt_protocol::firmware_v1 as pb;
use iomt_protocol::link::ProtoRange;

use crate::device::{DeviceConfig, OutOfRange};
use crate::gateway::GatewayConfig;
use crate::physio::{Body, HighHrEpisode};
use crate::wire::Frame;

#[derive(Parser)]
#[command(
    version,
    about = "Simulated IoMT wearables streaming like the BLE link"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Receive wearables: handshake, decode, timestamp, validate, acknowledge.
    Gateway {
        /// Address to listen on.
        #[arg(long, default_value = "127.0.0.1:7000")]
        listen: String,
        /// Oldest link protocol version accepted.
        #[arg(long, default_value_t = 1)]
        proto_min: u32,
        /// Newest link protocol version accepted.
        #[arg(long, default_value_t = 1)]
        proto_max: u32,
        /// Acknowledge after this many new records.
        #[arg(long, default_value_t = 10)]
        ack_every: u32,
    },
    /// Run wearables that sample once per second and stream to the gateway.
    Device {
        /// Gateway address.
        #[arg(long, default_value = "127.0.0.1:7000")]
        connect: String,
        /// Number of wearables.
        #[arg(long, default_value_t = 1)]
        count: u32,
        /// Base seed; the same seed always gives the same wearers.
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// What happens to the wearers.
        #[arg(long, value_enum, default_value_t = Scenario::Normal)]
        scenario: Scenario,
        /// Out-of-range start, seconds after launch.
        #[arg(long, default_value_t = 60)]
        out_of_range_at: u32,
        /// Out-of-range duration in seconds (over 900 overflows the ring).
        #[arg(long, default_value_t = 120)]
        out_of_range_for: u32,
        /// High heart rate start, seconds after launch.
        #[arg(long, default_value_t = 30)]
        high_hr_at: u32,
        /// High heart rate duration in seconds.
        #[arg(long, default_value_t = 120)]
        high_hr_for: u32,
        /// Oldest link protocol version the firmware speaks.
        #[arg(long, default_value_t = 1)]
        proto_min: u32,
        /// Newest link protocol version the firmware speaks.
        #[arg(long, default_value_t = 1)]
        proto_max: u32,
        /// Stop after this many seconds (default: run until Ctrl+C).
        #[arg(long)]
        duration: Option<u32>,
    },
    /// Print ten seconds of vitals and one LinkAck as framed on the wire.
    Demo,
}

#[derive(Clone, Copy, ValueEnum)]
enum Scenario {
    /// Rest, walk and run with normal vitals.
    Normal,
    /// Leave the gateway's range for a while, then come back and resend.
    OutOfRange,
    /// Heart rate above the 140 bpm alert threshold for a while.
    HighHr,
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Gateway {
            listen,
            proto_min,
            proto_max,
            ack_every,
        } => gateway::run(
            &listen,
            GatewayConfig {
                proto: ProtoRange {
                    min: proto_min,
                    max: proto_max,
                },
                ack_every_records: ack_every.max(1),
            },
        ),
        Command::Device {
            connect,
            count,
            seed,
            scenario,
            out_of_range_at,
            out_of_range_for,
            high_hr_at,
            high_hr_for,
            proto_min,
            proto_max,
            duration,
        } => {
            let out_of_range = matches!(scenario, Scenario::OutOfRange).then_some(OutOfRange {
                at_s: out_of_range_at,
                for_s: out_of_range_for,
            });
            let high_hr = matches!(scenario, Scenario::HighHr).then_some(HighHrEpisode {
                at_s: high_hr_at,
                for_s: high_hr_for,
            });
            let handles: Vec<_> = (0..count.max(1))
                .map(|index| {
                    let cfg = DeviceConfig {
                        index,
                        gateway: connect.clone(),
                        seed,
                        proto: ProtoRange {
                            min: proto_min,
                            max: proto_max,
                        },
                        out_of_range,
                        high_hr,
                        duration_s: duration,
                    };
                    thread::spawn(move || device::run(&cfg))
                })
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join()
                        .unwrap_or_else(|_| Err(std::io::Error::other("device thread panicked")))
                })
                .find_map(Result::err)
                .map_or(Ok(()), Err)
        }
        Command::Demo => {
            demo();
            Ok(())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("device-sim: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Ten seconds of vitals, then one LinkAck framed and read back.
fn demo() {
    let episode = HighHrEpisode { at_s: 3, for_s: 5 };
    let mut body = Body::new(42, Some(episode));
    for t_s in 0..10 {
        let r = body.step(t_s);
        println!(
            "t={t_s:2}s  HR {:3} bpm  SpO2 {:3} %  steps {}  {:?}  q {}",
            r.hr_bpm, r.spo2_pct, r.steps, r.activity, r.quality
        );
    }

    let ack = wire::link_frame(pb::link_control::Msg::Ack(pb::LinkAck {
        session_id: 7,
        through_seq: 120,
    }));
    let mut bytes = Vec::new();
    if let Err(e) = wire::write_frame(&mut bytes, &ack) {
        eprintln!("cannot frame the ack: {e}");
        return;
    }
    println!(
        "\nLinkAck on the wire ({} bytes): {bytes:02x?}",
        bytes.len()
    );
    match wire::read_frame(&mut bytes.as_slice()) {
        Ok(Frame::LinkControl(payload)) => match wire::decode_link(&payload) {
            Ok(msg) => println!("read back: {msg:?}"),
            Err(e) => eprintln!("cannot decode: {e}"),
        },
        Ok(other) => eprintln!("unexpected frame: {other:?}"),
        Err(e) => eprintln!("cannot read the frame: {e}"),
    }
}
