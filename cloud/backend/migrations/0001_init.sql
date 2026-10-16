-- Initial schema of the cloud backend (MVP-05, contracts 0.2.0).
--
-- Times are TIMESTAMPTZ (UTC); the API converts them to epoch milliseconds.
-- Device ids are pseudonymous: 32 lowercase hex characters computed on the gateway.

CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Accounts --------------------------------------------------------------------

CREATE TABLE users (
    id            UUID PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE CHECK (length(email) <= 256),
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL CHECK (role IN ('patient', 'clinician', 'admin')),
    display_name  TEXT CHECK (length(display_name) <= 64),
    subject_code  TEXT UNIQUE CHECK (subject_code ~ '^[A-Z]{2}-[0-9]{3,6}$'),
    disabled      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (subject_code IS NULL OR role = 'patient')
);
COMMENT ON TABLE users IS 'Accounts. Personal data is limited to email, display name and a pseudonymous subject code (07 §3.8).';
COMMENT ON COLUMN users.password_hash IS 'argon2id PHC string; the clear password is never stored or logged.';
COMMENT ON COLUMN users.subject_code IS 'Code shown to clinicians instead of a name, e.g. BN-017; patients only.';

CREATE TABLE refresh_tokens (
    id         UUID PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    family_id  UUID NOT NULL,
    token_hash BYTEA NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at    TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE refresh_tokens IS 'Rotating refresh tokens; one family per login. Reusing a used token revokes the family (REQ-SEC-010).';
COMMENT ON COLUMN refresh_tokens.token_hash IS 'SHA-256 of the 256-bit opaque token; the token itself is never stored.';
CREATE INDEX refresh_tokens_family ON refresh_tokens (family_id);

-- Devices ---------------------------------------------------------------------

CREATE TABLE devices (
    device_id        TEXT PRIMARY KEY CHECK (device_id ~ '^[0-9a-f]{32}$'),
    kind             TEXT NOT NULL CHECK (kind IN ('wearable', 'gateway')),
    status           TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'active', 'revoked')),
    label            TEXT CHECK (length(label) <= 64),
    assigned_user_id UUID REFERENCES users (id) ON DELETE SET NULL,
    fw_version       TEXT CHECK (length(fw_version) <= 32),
    last_seen        TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE devices IS 'Wearables and gateways. A wearable is assigned to at most one patient; many devices may share a patient.';
COMMENT ON COLUMN devices.last_seen IS 'Timestamp of the newest record received for this device.';
CREATE INDEX devices_assigned ON devices (assigned_user_id);

-- Telemetry (hypertables) -----------------------------------------------------

CREATE TABLE vitals (
    device_id      TEXT NOT NULL,
    window_start   TIMESTAMPTZ NOT NULL,
    seq            BIGINT NOT NULL CHECK (seq >= 0),
    boot_id        BIGINT NOT NULL CHECK (boot_id > 0),
    ts             TIMESTAMPTZ NOT NULL,
    hr_bpm         SMALLINT CHECK (hr_bpm BETWEEN 20 AND 250),
    spo2_pct       SMALLINT CHECK (spo2_pct BETWEEN 50 AND 100),
    activity       TEXT NOT NULL CHECK (activity IN ('rest', 'walk', 'run', 'unknown')),
    signal_quality SMALLINT NOT NULL CHECK (signal_quality BETWEEN 0 AND 100),
    received_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (device_id, window_start, seq, boot_id)
);
COMMENT ON TABLE vitals IS 'Summary vitals, about one row per second per wearable (D18). The unique key is the telemetry RecordKey, so a resend is stored once.';
COMMENT ON COLUMN vitals.hr_bpm IS 'Heart rate, beats per minute, 20..=250; NULL when the sensor could not estimate it.';
COMMENT ON COLUMN vitals.spo2_pct IS 'Oxygen saturation, percent, 50..=100; NULL when the sensor could not estimate it.';
COMMENT ON COLUMN vitals.signal_quality IS 'Confidence in the values, 0 (useless) to 100 (clean).';
-- Partitioned by window_start: TimescaleDB needs the partition column in every
-- unique index, and the RecordKey already contains it.
SELECT create_hypertable('vitals', by_range('window_start', INTERVAL '1 day'));
CREATE INDEX vitals_device_ts ON vitals (device_id, ts DESC);

CREATE TABLE windows (
    device_id           TEXT NOT NULL,
    window_start        TIMESTAMPTZ NOT NULL,
    seq                 BIGINT NOT NULL CHECK (seq >= 0),
    boot_id             BIGINT NOT NULL CHECK (boot_id > 0),
    window_end          TIMESTAMPTZ NOT NULL,
    sample_count        INTEGER NOT NULL CHECK (sample_count >= 0),
    hr_mean_bpm         REAL,
    hr_min_bpm          SMALLINT,
    hr_max_bpm          SMALLINT,
    spo2_min_pct        SMALLINT,
    hrv_rmssd_ms        REAL,
    step_count          INTEGER NOT NULL CHECK (step_count >= 0),
    dominant_activity   TEXT NOT NULL CHECK (dominant_activity IN ('rest', 'walk', 'run', 'unknown')),
    mean_signal_quality SMALLINT NOT NULL CHECK (mean_signal_quality BETWEEN 0 AND 100),
    model_version       TEXT,
    anomaly_score       REAL CHECK (anomaly_score BETWEEN 0 AND 1),
    anomaly_flag        BOOLEAN,
    received_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (device_id, window_start, seq, boot_id),
    CHECK (window_end > window_start)
);
COMMENT ON TABLE windows IS 'Features of one 30-120 s analysis window computed at the edge, with the optional edge model result.';
COMMENT ON COLUMN windows.hrv_rmssd_ms IS 'Heart rate variability as RMSSD, milliseconds.';
SELECT create_hypertable('windows', by_range('window_start', INTERVAL '7 days'));
CREATE INDEX windows_device_start ON windows (device_id, window_start DESC);

-- Compression after 7 days and retention of 180 days (08 §3.5).
ALTER TABLE vitals SET (timescaledb.compress, timescaledb.compress_segmentby = 'device_id');
SELECT add_compression_policy('vitals', INTERVAL '7 days');
SELECT add_retention_policy('vitals', INTERVAL '180 days');
ALTER TABLE windows SET (timescaledb.compress, timescaledb.compress_segmentby = 'device_id');
SELECT add_compression_policy('windows', INTERVAL '7 days');
SELECT add_retention_policy('windows', INTERVAL '180 days');

CREATE TABLE quarantine (
    id          BIGSERIAL PRIMARY KEY,
    device_id   TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('vitals', 'window')),
    ts          TIMESTAMPTZ,
    reason      TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE quarantine IS 'Records whose timestamp is too far from the server clock; kept for inspection, never shown as vitals.';

-- Alerts and clinical work ----------------------------------------------------

CREATE TABLE alerts (
    id              UUID PRIMARY KEY,
    device_id       TEXT NOT NULL REFERENCES devices (device_id),
    patient_id      UUID REFERENCES users (id) ON DELETE SET NULL,
    alert_type      TEXT NOT NULL CHECK (alert_type IN ('hr_high', 'hr_low', 'spo2_low', 'anomaly', 'device_offline', 'battery_low')),
    severity        TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    status          TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'acknowledged', 'resolved')),
    raised_at       TIMESTAMPTZ NOT NULL,
    value           DOUBLE PRECISION,
    acknowledged_at TIMESTAMPTZ,
    acknowledged_by UUID REFERENCES users (id) ON DELETE SET NULL,
    resolved_at     TIMESTAMPTZ,
    escalated_at    TIMESTAMPTZ,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE alerts IS 'Clinical and device alerts. At most one unresolved alert per device and type (dedup).';
COMMENT ON COLUMN alerts.value IS 'Value that crossed the threshold, in the unit of the alert type (bpm or percent).';
COMMENT ON COLUMN alerts.escalated_at IS 'When an unacknowledged alert was raised to the next level (REQ-REL-036).';
CREATE UNIQUE INDEX alerts_one_unresolved ON alerts (device_id, alert_type) WHERE status <> 'resolved';
CREATE INDEX alerts_raised ON alerts (raised_at DESC);

CREATE TABLE clinical_notes (
    id         UUID PRIMARY KEY,
    patient_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    author_id  UUID NOT NULL REFERENCES users (id),
    text       TEXT NOT NULL CHECK (length(text) BETWEEN 1 AND 1000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE clinical_notes IS 'Append-only clinician notes about a patient; a correction is a new note.';
CREATE INDEX clinical_notes_patient ON clinical_notes (patient_id, created_at DESC);

CREATE TABLE consents (
    id              UUID PRIMARY KEY,
    subject_user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    scope           TEXT NOT NULL CHECK (scope IN ('monitoring', 'research_export', 'model_training')),
    granted         BOOLEAN NOT NULL,
    form_version    TEXT NOT NULL CHECK (length(form_version) <= 32),
    recorded_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    recorded_by     UUID REFERENCES users (id) ON DELETE SET NULL
);
COMMENT ON TABLE consents IS 'Consent decisions, append-only; the newest record per scope is in force.';

CREATE TABLE models (
    version    TEXT PRIMARY KEY,
    sha256     TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    min_fw     TEXT,
    notes      TEXT CHECK (length(notes) <= 256),
    is_target  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE models IS 'Signed edge model versions; exactly one may be the target gateways should run.';
CREATE UNIQUE INDEX models_one_target ON models (is_target) WHERE is_target;

-- Audit log -------------------------------------------------------------------

CREATE TABLE audit_log (
    id        BIGSERIAL PRIMARY KEY,
    at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    actor_id  UUID,
    action    TEXT NOT NULL CHECK (length(action) <= 64),
    target    TEXT NOT NULL DEFAULT '' CHECK (length(target) <= 128),
    outcome   TEXT NOT NULL CHECK (outcome IN ('allowed', 'denied')),
    prev_hash BYTEA NOT NULL,
    hash      BYTEA NOT NULL
);
COMMENT ON TABLE audit_log IS 'Append-only audit trail. hash = SHA-256(prev_hash || entry), so an edited or deleted row breaks the chain (REQ-SEC-021).';

CREATE FUNCTION audit_log_append_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is append-only';
END;
$$;
CREATE TRIGGER audit_log_no_change BEFORE UPDATE OR DELETE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION audit_log_append_only();
