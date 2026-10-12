//! `device-sim`: simulated wearables and a gateway stub, linked over TCP.
//!
//! ```text
//! device-sim gateway                       # terminal 1: receive and print
//! device-sim device --count 3              # terminal 2: three wearables
//! device-sim device --scenario out-of-range --out-of-range-at 60 --out-of-range-for 120
//! ```
//!
//! The devices behave like the firmware (ring buffer, handshake, acks) and send
//! real 16-byte records and `iomt.firmware.v1.LinkControl` messages, so the data
//! has the same shape as from hardware. See `wire` for the TCP framing that
//! replaces BLE.

mod device;
mod gateway;
mod physio;
mod wire;

use std::process::ExitCode;
use std::thread;

use clap::{Parser, Subcommand, ValueEnum};
use iomt_protocol::link::ProtoRange;

use crate::device::{DeviceConfig, OutOfRange};
use crate::gateway::GatewayConfig;
use crate::physio::HighHrEpisode;

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
    },
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
        } => gateway::run(&GatewayConfig {
            listen,
            proto: ProtoRange {
                min: proto_min,
                max: proto_max,
            },
            ack_every_records: ack_every.max(1),
        }),
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
                .find(Result::is_err)
                .unwrap_or(Ok(()))
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
