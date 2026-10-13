//! `device-sim`: simulated wearables and a gateway stub, linked over TCP.
//!
//! ```text
//! device-sim gateway     # run the gateway stub on 127.0.0.1:7000
//! device-sim             # demo: ten seconds of vitals and one framed LinkAck
//! ```

mod gateway;
mod physio;
mod wire;

use std::process::ExitCode;

use iomt_protocol::firmware_v1 as pb;
use iomt_protocol::link::ProtoRange;

use crate::gateway::GatewayConfig;
use crate::physio::{Body, HighHrEpisode};
use crate::wire::Frame;

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("gateway") {
        let cfg = GatewayConfig {
            proto: ProtoRange { min: 1, max: 1 },
            ack_every_records: 10,
        };
        if let Err(e) = gateway::run("127.0.0.1:7000", cfg) {
            eprintln!("device-sim: {e}");
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }
    demo();
    ExitCode::SUCCESS
}

/// Steps 2a and 2c: ten seconds of vitals, then one LinkAck framed and read back.
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

    // 2c: frame a LinkAck as the gateway would send it, then read it back.
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
