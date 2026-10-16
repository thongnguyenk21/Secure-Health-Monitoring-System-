import type { Alert, MeasurementBucket } from "../../api/types";
import { vi } from "../../i18n/vi";
import { formatTime } from "../../lib/format";

/** Heart rate drawn as the dashed threshold line, in bpm. */
export const HR_THRESHOLD_BPM = 140;
// Steps per one-minute bucket: brisk walking is about 100/min, running 150+/min.
const WALK_STEPS_PER_MIN = 40;
const RUN_STEPS_PER_MIN = 140;
// A gap longer than this between buckets breaks the line instead of bridging it.
const MAX_GAP_MS = 5 * 60_000;
const MINUTE_MS = 60_000;

const W = 640;
const H = 150;
const PAD_L = 34;
const PAD_R = 8;
const PAD_T = 8;
const PAD_B = 18;

/** Shaded time span of one activity class. */
interface Band {
  x: number;
  width: number;
  kind: "walk" | "run";
}

/** Everything the SVG needs, in viewBox coordinates. */
export interface ChartModel {
  /** Polylines of the average heart rate, split at gaps. */
  lines: string[];
  /** Activity bands behind the line. */
  bands: Band[];
  /** Red dots at alert times. */
  dots: { x: number; y: number; id: string }[];
  /** Horizontal grid lines with their bpm label. */
  yTicks: { y: number; label: string }[];
  /** Time labels along the bottom. */
  xTicks: { x: number; label: string }[];
  /** Y of the threshold line. */
  thresholdY: number;
}

function activityOf(bucket: MeasurementBucket): Band["kind"] | null {
  const steps = bucket.step_count ?? 0;
  if (steps >= RUN_STEPS_PER_MIN) return "run";
  if (steps >= WALK_STEPS_PER_MIN) return "walk";
  return null;
}

/**
 * Turn the buckets and alerts of a 24-hour window into chart geometry.
 *
 * @param buckets - One-minute buckets, oldest first.
 * @param alerts - Alerts of the device; those outside the window are skipped.
 * @param fromMs - Window start, UTC epoch ms.
 * @param toMs - Window end, UTC epoch ms.
 * @returns The chart model.
 */
export function buildChartModel(
  buckets: readonly MeasurementBucket[],
  alerts: readonly Alert[],
  fromMs: number,
  toMs: number,
): ChartModel {
  const values = buckets.map((b) => b.hr_avg_bpm).filter((v): v is number => typeof v === "number");
  const lo = values.length ? Math.min(...values) : 60;
  const hi = values.length ? Math.max(...values) : 100;
  const yMin = Math.min(40, Math.floor(lo / 20) * 20);
  const yMax = Math.max(160, Math.ceil(hi / 20) * 20);
  const x = (t: number) => PAD_L + ((t - fromMs) / (toMs - fromMs)) * (W - PAD_L - PAD_R);
  const y = (v: number) => PAD_T + (1 - (Math.min(Math.max(v, yMin), yMax) - yMin) / (yMax - yMin)) * (H - PAD_T - PAD_B);

  const lines: string[] = [];
  let current: string[] = [];
  let lastT: number | null = null;
  for (const b of buckets) {
    const v = b.hr_avg_bpm;
    const broken = lastT !== null && b.bucket_start_unix_ms - lastT > MAX_GAP_MS;
    if (typeof v !== "number" || broken) {
      if (current.length > 1) lines.push(current.join(" "));
      current = [];
    }
    if (typeof v === "number") {
      current.push(`${x(b.bucket_start_unix_ms).toFixed(1)},${y(v).toFixed(1)}`);
      lastT = b.bucket_start_unix_ms;
    }
  }
  if (current.length > 1) lines.push(current.join(" "));

  const bands: Band[] = [];
  for (const b of buckets) {
    const kind = activityOf(b);
    if (!kind) continue;
    const bx = x(b.bucket_start_unix_ms);
    const bw = x(b.bucket_start_unix_ms + MINUTE_MS) - bx;
    const prev = bands[bands.length - 1];
    if (prev && prev.kind === kind && Math.abs(prev.x + prev.width - bx) < 0.5) prev.width += bw;
    else bands.push({ x: bx, width: bw, kind });
  }

  const nearestAvg = (t: number): number | null => {
    let best: MeasurementBucket | null = null;
    for (const b of buckets) {
      if (typeof b.hr_avg_bpm !== "number") continue;
      if (!best || Math.abs(b.bucket_start_unix_ms - t) < Math.abs(best.bucket_start_unix_ms - t)) best = b;
    }
    return best?.hr_avg_bpm ?? null;
  };
  const dots = alerts
    .filter((a) => a.raised_at_unix_ms >= fromMs && a.raised_at_unix_ms <= toMs)
    .map((a) => {
      const isHr = a.alert_type === "hr_high" || a.alert_type === "hr_low";
      const v = isHr && typeof a.value === "number" ? a.value : (nearestAvg(a.raised_at_unix_ms) ?? HR_THRESHOLD_BPM);
      return { x: x(a.raised_at_unix_ms), y: y(v), id: a.id };
    });

  const yTicks: ChartModel["yTicks"] = [];
  for (let v = yMin + 20; v <= yMax; v += 40) yTicks.push({ y: y(v), label: String(v) });
  const xTicks: ChartModel["xTicks"] = [0, 1, 2, 3, 4].map((i) => {
    const t = fromMs + (i * (toMs - fromMs)) / 4;
    return { x: x(t), label: formatTime(t) };
  });

  return { lines, bands, dots, yTicks, xTicks, thresholdY: y(HR_THRESHOLD_BPM) };
}

/**
 * 24-hour heart-rate chart in plain SVG: average line, activity bands,
 * threshold and alert dots.
 *
 * @param props - Buckets, alerts and the time window.
 */
export function HeartRateChart({
  buckets,
  alerts,
  fromMs,
  toMs,
}: {
  buckets: readonly MeasurementBucket[];
  alerts: readonly Alert[];
  fromMs: number;
  toMs: number;
}) {
  const m = buildChartModel(buckets, alerts, fromMs, toMs);
  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" className="chart" role="img" aria-label={vi.detail.chartAria}>
      {m.bands.map((b) => (
        <rect
          key={`${b.kind}-${b.x}`}
          x={b.x}
          y={PAD_T}
          width={Math.max(b.width, 1)}
          height={H - PAD_T - PAD_B}
          fill={b.kind === "run" ? "var(--rust-bg)" : "var(--accent-bg)"}
        />
      ))}
      <g stroke="var(--line)">
        {m.yTicks.map((t) => (
          <line key={t.label} x1={PAD_L} y1={t.y} x2={W - PAD_R} y2={t.y} />
        ))}
      </g>
      <line x1={PAD_L} y1={m.thresholdY} x2={W - PAD_R} y2={m.thresholdY} stroke="var(--alert)" strokeDasharray="5 4" />
      <g className="chart-label">
        {m.yTicks.map((t) => (
          <text key={t.label} x={PAD_L - 6} y={t.y + 3} textAnchor="end">
            {t.label}
          </text>
        ))}
        {m.xTicks.map((t, i) => (
          <text key={t.x} x={t.x} y={H - 4} textAnchor={i === 0 ? "start" : i === 4 ? "end" : "middle"}>
            {t.label}
          </text>
        ))}
      </g>
      {m.lines.map((points) => (
        <polyline key={points.slice(0, 24)} fill="none" stroke="var(--accent)" strokeWidth={2} points={points} />
      ))}
      {m.dots.map((d) => (
        <circle key={d.id} cx={d.x} cy={d.y} r={5} fill="var(--alert)" />
      ))}
    </svg>
  );
}
