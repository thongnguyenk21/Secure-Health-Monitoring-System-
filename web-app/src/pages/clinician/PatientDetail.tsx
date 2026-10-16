import { useState, type FormEvent } from "react";
import {
  useAckAlert,
  useCreateNote,
  useDayBuckets,
  useDeviceAlerts,
  useNotes,
  usePatient,
} from "../../api/queries";
import type { Alert, PatientSummary } from "../../api/types";
import { Empty, errorText, Pill, QueryStatus } from "../../components/ui";
import { vi } from "../../i18n/vi";
import { formatDateTime, formatOptional, formatTime, formatWeekday, shortDeviceId } from "../../lib/format";
import { activityLabel, alertStatusLabel, alertTypeLabel, severityColor, severityLabel } from "../../lib/status";
import { HeartRateChart } from "./HeartRateChart";

const WEEK_MS = 7 * 24 * 60 * 60_000;

/**
 * Value of an alert with its unit, for example `152 bpm` or `88 %`.
 *
 * @param alert - Alert.
 * @returns The text, or null when the alert has no value.
 */
export function formatAlertValue(alert: Alert): string | null {
  if (alert.value === null || alert.value === undefined) return null;
  if (alert.alert_type === "hr_high" || alert.alert_type === "hr_low") {
    return `${Math.round(alert.value)} ${vi.common.bpm}`;
  }
  if (alert.alert_type === "spo2_low" || alert.alert_type === "battery_low") {
    return `${Math.round(alert.value)}${vi.common.percent}`;
  }
  return String(alert.value);
}

function Header({ patient }: { patient: PatientSummary }) {
  const needsAction = patient.status === "critical" || patient.status === "warning";
  const seen = patient.last_seen_unix_ms ? formatDateTime(patient.last_seen_unix_ms) : vi.detail.neverSeen;
  return (
    <div className="row-between">
      <div>
        <h2 className="detail-title">{vi.detail.title(patient.subject_code, patient.display_name)}</h2>
        <div className="muted small">
          {patient.device_id ? vi.detail.deviceLine(shortDeviceId(patient.device_id), seen) : vi.detail.noDeviceLine}
        </div>
      </div>
      {needsAction ? (
        <Pill variant={patient.status === "critical" ? "alert" : "warn"}>{vi.detail.needsAction}</Pill>
      ) : null}
    </div>
  );
}

function VitalTiles({ patient }: { patient: PatientSummary }) {
  const top = patient.top_alert?.alert_type;
  const hrAlert = top === "hr_high" || top === "hr_low";
  const spo2Alert = top === "spo2_low";
  const latest = patient.status === "no_device" ? null : patient.latest;
  return (
    <div className="tiles4">
      <div className="tile">
        <div className="cap">{vi.detail.tileHr}</div>
        <div className={`num big${hrAlert ? " text-alert" : ""}`}>{formatOptional(latest?.hr_bpm)}</div>
      </div>
      <div className="tile">
        <div className="cap">{vi.detail.tileSpo2}</div>
        <div className={`num big${spo2Alert ? " text-alert" : ""}`}>
          {formatOptional(latest?.spo2_pct, vi.common.percent)}
        </div>
      </div>
      <div className="tile">
        <div className="cap">{vi.detail.tileActivity}</div>
        <div className="activity">{latest ? activityLabel(latest.activity) : vi.common.dash}</div>
      </div>
      <div className="tile">
        <div className="cap">{vi.detail.tileHrv}</div>
        <div className="num big">{formatOptional(patient.hrv_rmssd_ms, vi.common.ms)}</div>
      </div>
    </div>
  );
}

function ChartTile({ deviceId, alerts }: { deviceId: string | null | undefined; alerts: readonly Alert[] }) {
  const day = useDayBuckets(deviceId);
  return (
    <div className="tile chart-tile">
      <div className="row-between small">
        <b>{vi.detail.chartTitle}</b>
        <span className="muted">{vi.detail.chartLegend}</span>
      </div>
      {!deviceId ? (
        <Empty>{vi.detail.chartNoDevice}</Empty>
      ) : day.data ? (
        day.data.buckets.length === 0 ? (
          <Empty>{vi.detail.chartEmpty}</Empty>
        ) : (
          <HeartRateChart buckets={day.data.buckets} alerts={alerts} fromMs={day.data.from} toMs={day.data.to} />
        )
      ) : (
        <QueryStatus isLoading={day.isLoading} error={day.error} />
      )}
    </div>
  );
}

function OpenAlerts({ alerts }: { alerts: readonly Alert[] }) {
  const ack = useAckAlert();
  return (
    <>
      <div className="cap section-cap">{vi.detail.openAlerts}</div>
      {alerts.length === 0 ? <Empty>{vi.detail.noOpenAlerts}</Empty> : null}
      {alerts.map((a) => {
        const value = formatAlertValue(a);
        const busy = ack.isPending && ack.variables === a.id;
        return (
          <div key={a.id} className="tile open-alert" style={{ borderLeftColor: severityColor(a.severity) }}>
            <div>
              <b>{alertTypeLabel(a.alert_type)}</b>
              {vi.common.sep}
              {formatTime(a.raised_at_unix_ms)}
              <div className="muted small">
                {value ? `${vi.detail.alertValue(value)}${vi.common.sep}` : null}
                {severityLabel(a.severity)}
              </div>
            </div>
            <Pill variant="alert" onClick={() => ack.mutate(a.id)} disabled={busy}>
              {busy ? vi.detail.acknowledging : vi.detail.acknowledge}
            </Pill>
          </div>
        );
      })}
      {ack.error ? <p className="error small">{errorText(ack.error)}</p> : null}
    </>
  );
}

function WeekAlerts({ alerts }: { alerts: readonly Alert[] }) {
  return (
    <div className="tile">
      <div className="cap tile-cap">{vi.detail.alerts7d}</div>
      {alerts.length === 0 ? <Empty>{vi.detail.noAlerts7d}</Empty> : null}
      <div className="scroll-list">
        {alerts.map((a) => (
          <div key={a.id} className="alog">
            <span title={formatDateTime(a.raised_at_unix_ms)}>{formatWeekday(a.raised_at_unix_ms)}</span>
            <span>{alertTypeLabel(a.alert_type)}</span>
            <em className={`st-${a.status}`}>{alertStatusLabel(a.status)}</em>
          </div>
        ))}
      </div>
    </div>
  );
}

function Notes({ userId }: { userId: string }) {
  const notes = useNotes(userId);
  const create = useCreateNote(userId);
  const [text, setText] = useState("");
  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    const trimmed = text.trim();
    if (!trimmed) return;
    create.mutate(trimmed, { onSuccess: () => setText("") });
  };
  return (
    <div className="tile">
      <div className="cap tile-cap">{vi.detail.notes}</div>
      <QueryStatus isLoading={notes.isLoading} error={notes.error} />
      {notes.data && notes.data.length === 0 ? <Empty>{vi.detail.noNotes}</Empty> : null}
      <div className="scroll-list">
        {notes.data?.map((n) => (
          <div key={n.id} className="note">
            <div>{n.text}</div>
            <div className="muted xsmall">
              {n.author_name ?? vi.detail.noteAuthorFallback}
              {vi.common.sep}
              {formatDateTime(n.created_at_unix_ms)}
            </div>
          </div>
        ))}
      </div>
      <form className="note-form" onSubmit={onSubmit}>
        <textarea
          value={text}
          maxLength={1000}
          rows={2}
          placeholder={vi.detail.notePlaceholder}
          aria-label={vi.detail.notePlaceholder}
          onChange={(e) => setText(e.target.value)}
        />
        <button type="submit" className="btn-primary" disabled={create.isPending || !text.trim()}>
          {create.isPending ? vi.detail.savingNote : vi.detail.saveNote}
        </button>
      </form>
      {create.error ? <p className="error small">{errorText(create.error)}</p> : null}
    </div>
  );
}

/**
 * Right column of the clinician screen: vitals, 24-hour chart, open alerts,
 * the last 7 days of alerts and the clinical notes of one patient.
 *
 * @param props - Patient user id.
 */
export function PatientDetail({ userId }: { userId: string }) {
  const patient = usePatient(userId);
  const deviceId = patient.data?.device_id;
  const alerts = useDeviceAlerts(deviceId);

  if (!patient.data) return <QueryStatus isLoading={patient.isLoading} error={patient.error} />;

  const all = alerts.data?.items ?? [];
  const open = all.filter((a) => a.status === "open");
  const weekStart = Date.now() - WEEK_MS;
  const week = all.filter((a) => a.raised_at_unix_ms >= weekStart);

  return (
    <div className="detail">
      <Header patient={patient.data} />
      <VitalTiles patient={patient.data} />
      <ChartTile deviceId={deviceId} alerts={all} />
      <OpenAlerts alerts={open} />
      {alerts.error ? <p className="error small">{errorText(alerts.error)}</p> : null}
      <div className="tiles2">
        <WeekAlerts alerts={week} />
        <Notes userId={userId} />
      </div>
    </div>
  );
}
