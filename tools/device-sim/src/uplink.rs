//! Optional upload of what the gateway stub receives to the cloud ingest (`--cloud`).
//!
//! Plays the part of edge-sync for development: summary vitals go up about once
//! per second in batches, and every 60 s window of a device becomes one
//! `WindowFeatures` row. Nothing is persisted: if the cloud is down, batches
//! wait in memory (bounded) and are retried; a real edge-sync uses SQLite.
//!
//! HRV here is a modelled value from the mean heart rate, not a measurement; the
//! real RMSSD comes from beat-to-beat intervals on the Pi.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use iomt_frames::Record;
use iomt_protocol::telemetry_v1 as pb;
use iomt_protocol::telemetry_v1::telemetry_ingest_service_client::TelemetryIngestServiceClient;
use iomt_protocol::{IdempotencyKey, SummaryVitals, UnixMs, vitals_to_pb};

/// Length of one analysis window, ms.
const WINDOW_MS: i64 = 60_000;
/// Most records per upload (the backend accepts 400).
const MAX_BATCH: usize = 200;
/// Most records kept while the cloud is unreachable.
const MAX_BACKLOG: usize = 20_000;

/// One received sample handed over by the gateway.
struct Item {
    boot_id: u32,
    seq: u32,
    steps: u16,
    vitals: SummaryVitals,
}

static SENDER: OnceLock<SyncSender<Item>> = OnceLock::new();

/// Starts the uploader thread; later calls are ignored.
///
/// # Arguments
/// * `endpoint` - gRPC address of the backend, e.g. `http://127.0.0.1:9443`.
pub fn install(endpoint: String) {
    let (tx, rx) = mpsc::sync_channel(MAX_BACKLOG);
    if SENDER.set(tx).is_ok() {
        println!("uplink: uploading to {endpoint}");
        thread::spawn(move || run(&endpoint, &rx));
    }
}

/// Queues one validated sample for upload; does nothing without `--cloud`.
pub fn submit(boot_id: u32, rec: &Record, vitals: &SummaryVitals) {
    if let Some(tx) = SENDER.get() {
        let _ = tx.try_send(Item {
            boot_id,
            seq: rec.seq,
            steps: rec.steps,
            vitals: vitals.clone(),
        });
    }
}

/// Running aggregate of one device's current window.
struct Window {
    start_ms: i64,
    first_seq: u32,
    boot_id: u32,
    last_ms: i64,
    count: u32,
    hr: Vec<u16>,
    spo2_min: Option<u8>,
    steps: u32,
    activities: [u32; 4],
    quality_sum: u32,
}

impl Window {
    fn new(start_ms: i64, item: &Item) -> Self {
        Self {
            start_ms,
            first_seq: item.seq,
            boot_id: item.boot_id,
            last_ms: start_ms,
            count: 0,
            hr: Vec::new(),
            spo2_min: None,
            steps: 0,
            activities: [0; 4],
            quality_sum: 0,
        }
    }

    fn add(&mut self, item: &Item) {
        let v = &item.vitals;
        self.count += 1;
        self.last_ms = self.last_ms.max(v.ts.0);
        self.hr.extend(v.hr_bpm);
        if let Some(s) = v.spo2_pct {
            self.spo2_min = Some(self.spo2_min.map_or(s, |m| m.min(s)));
        }
        self.steps += u32::from(item.steps);
        self.activities[pb::Activity::from(v.activity) as usize - 1] += 1;
        self.quality_sum += u32::from(v.quality.value());
    }

    fn finish(&self, device: &iomt_protocol::DeviceId) -> pb::WindowFeatures {
        let n = self.hr.len();
        let mean = (n > 0).then(|| self.hr.iter().map(|&h| f32::from(h)).sum::<f32>() / n as f32);
        // 1 Hz heart rates cannot give beat-to-beat intervals, so this is a
        // simple model, not a measurement: about 42 ms at 70 bpm, falling with
        // the square of the heart rate (vagal tone drops as the rate rises).
        let rmssd = mean.map(|hr| 42.0 * (70.0 / hr).powi(2));
        let dominant = (0..4).max_by_key(|&i| self.activities[i]).unwrap_or(3);
        pb::WindowFeatures {
            key: Some(
                IdempotencyKey {
                    device_id: device.clone(),
                    window_start: UnixMs(self.start_ms),
                    seq: self.first_seq,
                    boot_id: self.boot_id,
                }
                .into(),
            ),
            window_end_unix_ms: (self.last_ms + 1000).max(self.start_ms + 1),
            sample_count: self.count,
            hr_mean_bpm: mean,
            hr_min_bpm: self.hr.iter().min().map(|&h| u32::from(h)),
            hr_max_bpm: self.hr.iter().max().map(|&h| u32::from(h)),
            spo2_min_pct: self.spo2_min.map(u32::from),
            hrv_rmssd_ms: rmssd,
            step_count: self.steps,
            dominant_activity: i32::try_from(dominant + 1).unwrap_or(4),
            mean_signal_quality: self.quality_sum / self.count.max(1),
            inference: None,
        }
    }
}

fn run(endpoint: &str, rx: &Receiver<Item>) {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        println!("uplink: cannot start the async runtime; uploads disabled");
        return;
    };
    let mut client: Option<TelemetryIngestServiceClient<tonic::transport::Channel>> = None;
    let mut vitals: Vec<pb::SummaryVitals> = Vec::new();
    let mut windows: Vec<pb::WindowFeatures> = Vec::new();
    let mut open: HashMap<String, Window> = HashMap::new();
    let mut last_send = Instant::now();
    let (mut sent, mut rejected) = (0usize, 0usize);

    loop {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(item) => {
                let device = item.vitals.device_id.clone();
                let start = item.vitals.ts.0.div_euclid(WINDOW_MS) * WINDOW_MS;
                let w = open
                    .entry(device.as_str().to_owned())
                    .or_insert_with(|| Window::new(start, &item));
                if w.start_ms != start || w.boot_id != item.boot_id {
                    windows.push(w.finish(&device));
                    *w = Window::new(start, &item);
                }
                w.add(&item);
                let key = IdempotencyKey {
                    device_id: device,
                    window_start: UnixMs(start),
                    seq: item.seq,
                    boot_id: item.boot_id,
                };
                vitals.push(vitals_to_pb(key, &item.vitals));
                if vitals.len() > MAX_BACKLOG {
                    vitals.drain(..vitals.len() - MAX_BACKLOG);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if (vitals.is_empty() && windows.is_empty()) || last_send.elapsed() < Duration::from_secs(1)
        {
            continue;
        }
        last_send = Instant::now();
        let n_v = vitals.len().min(MAX_BATCH);
        let n_w = windows.len().min(MAX_BATCH - n_v);
        let request = pb::UploadBatchRequest {
            schema_version: "1".into(),
            sent_at_unix_ms: chrono_now_ms(),
            vitals: vitals[..n_v].to_vec(),
            windows: windows[..n_w].to_vec(),
        };
        let result = rt.block_on(async {
            if client.is_none() {
                client = Some(TelemetryIngestServiceClient::connect(endpoint.to_owned()).await?);
            }
            match client.as_mut() {
                Some(c) => Ok(c.upload_batch(request).await?.into_inner()),
                None => Err(Box::<dyn std::error::Error>::from("no client")),
            }
        });
        match result {
            Ok(resp) => {
                vitals.drain(..n_v);
                windows.drain(..n_w);
                sent += resp.accepted.len();
                rejected += resp.rejected.len();
                if let Some(r) = resp.rejected.first() {
                    println!(
                        "uplink: {} rejected, e.g. {}",
                        resp.rejected.len(),
                        r.detail
                    );
                }
                if sent % 100 < resp.accepted.len() {
                    println!("uplink: {sent} records accepted, {rejected} rejected");
                }
            }
            Err(e) => {
                client = None;
                println!(
                    "uplink: upload failed ({e}); {} records wait",
                    vitals.len() + windows.len()
                );
            }
        }
    }
}

fn chrono_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}
