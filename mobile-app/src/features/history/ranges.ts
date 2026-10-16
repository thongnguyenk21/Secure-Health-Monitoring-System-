import type { Resolution } from "../../api/types";
import { vi } from "../../i18n/vi";
import { formatDayMonth, formatTime, formatWeekday } from "../../utils/format";

/** History ranges of the segmented control. */
export type HistoryRange = "day" | "week" | "month";

/** How one history range is fetched and labelled. */
export interface RangeSpec {
  /** Range id. */
  id: HistoryRange;
  /** Label of the segment. */
  label: string;
  /** Length of the window ending now, in ms. */
  spanMs: number;
  /** Rollup the backend returns for this range. */
  resolution: Exclude<Resolution, "raw">;
  /** Bucket length of that rollup, in ms. */
  bucketMs: number;
  /**
   * Bin length the chart draws, in ms; a hole longer than three bins breaks the line.
   *
   * @remarks
   * A day has 1440 one-minute buckets, far more than a phone is wide; they are
   * averaged into 10-minute bins so the line stays readable.
   */
  displayBinMs: number;
  /** Axis label of a tick. */
  tickLabel: (unixMs: number) => string;
}

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** The three ranges: last 24 h by minute, last 7 days by hour, last 30 days by day. */
export const RANGES: Record<HistoryRange, RangeSpec> = {
  day: { id: "day", label: vi.history.day, spanMs: DAY, resolution: "1m", bucketMs: MINUTE, displayBinMs: 10 * MINUTE, tickLabel: formatTime },
  week: { id: "week", label: vi.history.week, spanMs: 7 * DAY, resolution: "1h", bucketMs: HOUR, displayBinMs: HOUR, tickLabel: formatWeekday },
  month: { id: "month", label: vi.history.month, spanMs: 30 * DAY, resolution: "1d", bucketMs: DAY, displayBinMs: DAY, tickLabel: formatDayMonth },
};

/** One point of the average heart-rate line. */
export interface HrPoint {
  /** Bucket start, UTC epoch ms. */
  t: number;
  /** Average heart rate of the bucket, in bpm. */
  hr: number;
}

/**
 * Average points into fixed time bins.
 *
 * @param points - Points sorted by time.
 * @param binMs - Bin length in ms; bins start at multiples of it.
 * @returns One point per non-empty bin, at the bin start, oldest first.
 */
export function downsample(points: readonly HrPoint[], binMs: number): HrPoint[] {
  const out: HrPoint[] = [];
  let binStart = Number.NaN;
  let sum = 0;
  let count = 0;
  for (const p of points) {
    const start = Math.floor(p.t / binMs) * binMs;
    if (start !== binStart) {
      if (count > 0) out.push({ t: binStart, hr: sum / count });
      binStart = start;
      sum = 0;
      count = 0;
    }
    sum += p.hr;
    count += 1;
  }
  if (count > 0) out.push({ t: binStart, hr: sum / count });
  return out;
}

/**
 * Split points into line segments wherever data is missing.
 *
 * @param points - Points sorted by time.
 * @param maxGapMs - Largest step that still joins two points.
 * @returns Runs of consecutive points.
 */
export function splitOnGaps(points: readonly HrPoint[], maxGapMs: number): HrPoint[][] {
  const runs: HrPoint[][] = [];
  let run: HrPoint[] = [];
  let prev: HrPoint | null = null;
  for (const p of points) {
    if (prev && p.t - prev.t > maxGapMs) {
      runs.push(run);
      run = [];
    }
    run.push(p);
    prev = p;
  }
  if (run.length > 0) runs.push(run);
  return runs;
}

/**
 * Y-axis bounds that hold every value, rounded to steps of 20 bpm.
 *
 * @param values - Heart rates in bpm.
 * @returns Lower and upper bound; 60–140 when there is nothing to show.
 */
export function hrAxis(values: readonly number[]): { min: number; max: number } {
  if (values.length === 0) return { min: 60, max: 140 };
  const lo = Math.min(...values);
  const hi = Math.max(...values);
  const min = Math.min(60, Math.floor(lo / 20) * 20);
  const max = Math.max(min + 80, Math.ceil((hi + 2) / 20) * 20);
  return { min, max };
}
