//! Integration tests against a real TimescaleDB.
//!
//! Set `IOMT_TEST_DATABASE_URL` to a server where the user may create databases,
//! e.g. `postgres://iomt:iomt@127.0.0.1:5432/iomt`. Each test creates and drops
//! its own database. Without the variable the tests pass without running, so
//! `cargo test --workspace` works on machines without Docker.

// Test helpers outside #[test] functions may unwrap too (clippy.toml allows it in tests).
#![allow(clippy::unwrap_used)]

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, StatusCode};
use iomt_backend::infra::jwt::JwtKeys;
use iomt_backend::ingest::Ingest;
use iomt_backend::state::AppState;
use iomt_backend::{MIGRATOR, api, seed};
use iomt_protocol::telemetry_v1 as pb;
use iomt_protocol::telemetry_v1::telemetry_ingest_service_server::TelemetryIngestService;
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection, PgPool};
use tower::ServiceExt;

const DEVICE: &str = "0123456789abcdef0123456789abcdef";

struct TestDb {
    admin_url: String,
    name: String,
    pool: PgPool,
}

impl TestDb {
    async fn create() -> Option<Self> {
        let admin_url = std::env::var("IOMT_TEST_DATABASE_URL").ok()?;
        let name = format!("iomt_test_{}", uuid::Uuid::new_v4().simple());
        let mut conn = PgConnection::connect(&admin_url).await.ok()?;
        sqlx::query(&format!("CREATE DATABASE {name}"))
            .execute(&mut conn)
            .await
            .ok()?;
        let base = admin_url
            .rsplit_once('/')
            .map_or(admin_url.as_str(), |(b, _)| b);
        let pool = PgPool::connect(&format!("{base}/{name}")).await.ok()?;
        MIGRATOR.run(&pool).await.ok()?;
        Some(Self {
            admin_url,
            name,
            pool,
        })
    }

    async fn drop(self) {
        self.pool.close().await;
        if let Ok(mut conn) = PgConnection::connect(&self.admin_url).await {
            let _ = sqlx::query(&format!(
                "DROP DATABASE IF EXISTS {} WITH (FORCE)",
                self.name
            ))
            .execute(&mut conn)
            .await;
        }
    }
}

fn state(pool: PgPool) -> AppState {
    AppState::new(
        pool,
        JwtKeys::load_or_generate(None).unwrap(),
        iomt_backend::config::DEFAULT_TENANT,
    )
    .unwrap()
}

fn batch(from_s: i64, to_s: i64, hr: u32) -> pb::UploadBatchRequest {
    let now = chrono::Utc::now().timestamp_millis() - 60_000;
    let vitals = (from_s..to_s)
        .map(|s| {
            let ts = now + s * 1000;
            pb::SummaryVitals {
                key: Some(pb::RecordKey {
                    device_id: DEVICE.into(),
                    window_start_unix_ms: ts - ts.rem_euclid(60_000),
                    seq: u32::try_from(s + 1).unwrap(),
                    boot_id: 7,
                }),
                ts_unix_ms: ts,
                hr_bpm: Some(hr),
                spo2_pct: Some(97),
                activity: pb::Activity::Rest as i32,
                signal_quality: 90,
            }
        })
        .collect();
    pb::UploadBatchRequest {
        schema_version: "1".into(),
        sent_at_unix_ms: now,
        vitals,
        windows: Vec::new(),
    }
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let body = if body.is_null() {
        Body::empty()
    } else {
        Body::from(body.to_string())
    };
    let resp = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn ingest_is_idempotent_and_opens_one_alert() {
    let Some(db) = TestDb::create().await else {
        eprintln!("IOMT_TEST_DATABASE_URL not set: skipped");
        return;
    };
    seed::seed_dev(&db.pool, &[DEVICE.to_owned()])
        .await
        .unwrap();
    let svc = Ingest::new(state(db.pool.clone()));

    let first = svc
        .upload_batch(tonic::Request::new(batch(0, 15, 150)))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(first.accepted.len(), 15);
    assert!(first.rejected.is_empty());
    // The same batch again: all accepted, nothing stored twice.
    let again = svc
        .upload_batch(tonic::Request::new(batch(0, 15, 150)))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(again.accepted.len(), 15);

    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM vitals")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(rows, 15);
    let alerts: Vec<(String, String)> = sqlx::query_as("SELECT alert_type, status FROM alerts")
        .fetch_all(&db.pool)
        .await
        .unwrap();
    assert_eq!(alerts, vec![("hr_high".to_owned(), "open".to_owned())]);
    let status: String = sqlx::query_scalar("SELECT status FROM devices WHERE device_id = $1")
        .bind(DEVICE)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        status, "active",
        "a pending wearable activates on its first upload"
    );
    db.drop().await;
}

#[tokio::test]
async fn unknown_devices_and_bad_values_are_rejected_per_record() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let svc = Ingest::new(state(db.pool.clone()));
    let resp = svc
        .upload_batch(tonic::Request::new(batch(0, 2, 70)))
        .await
        .unwrap()
        .into_inner();
    assert!(resp.accepted.is_empty());
    assert!(
        resp.rejected
            .iter()
            .all(|r| r.reason == pb::RejectReason::UnknownDevice as i32)
    );

    seed::seed_dev(&db.pool, &[DEVICE.to_owned()])
        .await
        .unwrap();
    let mut bad = batch(0, 1, 70);
    bad.vitals[0].hr_bpm = Some(999);
    let resp = svc
        .upload_batch(tonic::Request::new(bad))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        resp.rejected[0].reason,
        pb::RejectReason::InvalidValue as i32
    );
    db.drop().await;
}

#[tokio::test]
async fn roles_see_what_the_contract_allows() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    seed::seed_dev(&db.pool, &[DEVICE.to_owned()])
        .await
        .unwrap();
    let st = state(db.pool.clone());
    Ingest::new(st.clone())
        .upload_batch(tonic::Request::new(batch(0, 15, 150)))
        .await
        .unwrap();
    let app = api::router(st, None).layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 9))));

    let login = |email: &'static str| {
        let app = app.clone();
        async move {
            let (s, v) = call(
                &app,
                "POST",
                "/api/v1/auth/login",
                None,
                json!({"email": email, "password": seed::DEV_PASSWORD}),
            )
            .await;
            assert_eq!(s, StatusCode::OK);
            v["access_token"].as_str().unwrap().to_owned()
        }
    };
    let clinician = login("bs.minh@iomt.local").await;
    let admin = login("admin@iomt.local").await;

    let (s, patients) = call(
        &app,
        "GET",
        "/api/v1/patients",
        Some(&clinician),
        Value::Null,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(patients[0]["subject_code"], "BN-017");
    assert_eq!(patients[0]["status"], "warning");
    assert_eq!(patients[0]["top_alert"]["alert_type"], "hr_high");

    let (s, me) = call(&app, "GET", "/api/v1/me", Some(&clinician), Value::Null).await;
    assert_eq!(
        (s, me["display_name"].as_str()),
        (StatusCode::OK, Some("BS. Minh"))
    );

    let (s, _) = call(&app, "GET", "/api/v1/patients", Some(&admin), Value::Null).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "admins see no health data");
    let (s, _) = call(&app, "GET", "/api/v1/users", Some(&clinician), Value::Null).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, users) = call(
        &app,
        "GET",
        "/api/v1/users?limit=2",
        Some(&admin),
        Value::Null,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(users["next_cursor"].is_string());

    let alert_id = patients[0]["top_alert"]["id"].as_str().unwrap().to_owned();
    let (s, acked) = call(
        &app,
        "POST",
        &format!("/api/v1/alerts/{alert_id}/ack"),
        Some(&clinician),
        Value::Null,
    )
    .await;
    assert_eq!(
        (s, acked["status"].as_str()),
        (StatusCode::OK, Some("acknowledged"))
    );

    let (s, err) = call(
        &app,
        "GET",
        "/api/v1/alerts?limit=abc",
        Some(&clinician),
        Value::Null,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(err["code"], "validation_failed");

    let chain_ok: bool = sqlx::query_scalar(
        "SELECT bool_and(a.prev_hash = b.hash) FROM audit_log a JOIN audit_log b ON b.id = (SELECT max(id) FROM audit_log WHERE id < a.id)",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(chain_ok, "each audit row links to the previous one");
    db.drop().await;
}
