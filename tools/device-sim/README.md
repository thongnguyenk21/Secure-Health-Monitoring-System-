# tools/device-sim — simulated wearables and gateway stub

- **Purpose:** Run wearables without hardware. Each simulated device samples once per second, keeps samples in the same offline ring buffer as the firmware (`offline-buffer`, 900 samples), sends real 16-byte `iomt-frames` records and `iomt.firmware.v1.LinkControl` messages, and only forgets a sample when the gateway acknowledges it. A gateway stub receives them so the flow can be seen end to end.
- **Tier:** `[DEV]` tool
- **Scope:** `[MVP]`
- **Owner:** RW-04 (MVP-02)
- **Tech:** Rust, std threads, TCP.

## Run

```bash
# terminal 1: the gateway stub
cargo run -p device-sim -- gateway

# terminal 2: one wearable with normal activity
cargo run -p device-sim -- device

# three wearables
cargo run -p device-sim -- device --count 3

# walk out of range after 60 s for 2 min, then resend the backlog
cargo run -p device-sim -- device --scenario out-of-range --out-of-range-at 60 --out-of-range-for 120

# heart rate above the 140 bpm alert threshold for 2 min, starting after 30 s
cargo run -p device-sim -- device --scenario high-hr

# protocol mismatch: the gateway refuses the wearable and raises an alarm
cargo run -p device-sim -- device --proto-min 2 --proto-max 3
```

`--seed` makes a wearer reproducible: the same seed gives the same vitals.
An out-of-range window longer than 900 s overflows the ring; the device then
reports lost samples and the gateway logs the gap in sequence numbers.

## What each side does

| Device (`device`) | Gateway stub (`gateway`) |
|---|---|
| Sends its 6-byte address first (stands in for the BLE address) | Derives the pseudonymous `DeviceId` = first 16 bytes of SHA-256 of the address |
| Answers `LinkHello` at once with `LinkReport` (`mono0_us`, firmware, protocol range, battery) | Sends `LinkHello` on connect and every 10 min, negotiates with `link::negotiate`, replies `LinkSelect` or refuses and logs an alarm |
| Records every second into the ring, connected or not | Turns `mono_ms` into UTC with the handshake offset |
| Streams unsent records oldest-first while the link is up | Checks the CRC, drops duplicates across reconnections, validates as `SummaryVitals` |
| Removes records on `LinkAck` | Sends `LinkAck` every 10 records or 5 s |

The stub stops there. Windowing, inference, the SQLite queue and the upload are
edge-ingest and edge-sync work (MVP-04).

## TCP framing (replaces BLE)

```text
offset size field
  0     1   kind      1 = peer address, 2 = link control, 3 = record
  1     2   length    payload length, u16 little endian, <= 512
  3     n   payload   6-byte address | protobuf LinkControl | 16-byte record
```

Defined in `src/wire.rs`. A later edge-ingest can accept this transport next to
BLE, so the simulator also drives the real gateway service.
