import type { Alert, Measurement } from "../../api/types";
import { vi } from "../../i18n/vi";
import { colors } from "../../theme/colors";
import { formatTime } from "../../utils/format";
import { alertTitle, alertValueText } from "../alerts/alertText";

/** A reading older than this, in ms, means the watch is no longer connected. */
export const FRESH_MS = 2 * 60_000;

/** Visual tone of the status card. */
export type StatusTone = "ok" | "warn" | "alert" | "muted";

/** What the home status card shows. */
export interface HealthStatus {
  /** Tone that picks the card colours and icon. */
  tone: StatusTone;
  /** Bold first line, e.g. "Sức khoẻ: bình thường". */
  title: string;
  /** Short context line, e.g. "Nhịp tim 92 hợp với lúc đi bộ". */
  context: string;
}

/** Colours of one tone: strong colour, soft background, icon glyph. */
export interface ToneStyle {
  /** Border and icon background. */
  color: string;
  /** Card background. */
  background: string;
  /** Glyph drawn in the round icon. */
  glyph: string;
}

/**
 * Colours of each status tone.
 *
 * @param tone - The tone.
 * @returns Strong colour, soft background and icon glyph.
 */
export function toneStyle(tone: StatusTone): ToneStyle {
  switch (tone) {
    case "ok":
      return { color: colors.ok, background: colors.okBg, glyph: "✓" };
    case "warn":
      return { color: colors.warn, background: colors.warnBg, glyph: "!" };
    case "alert":
      return { color: colors.alert, background: colors.alertBg, glyph: "!" };
    default:
      return { color: colors.muted, background: colors.paper, glyph: "?" };
  }
}

/**
 * Tell whether a reading is recent enough to call the watch connected.
 *
 * @param latest - Latest reading, or null.
 * @param nowMs - Current time, UTC epoch ms.
 * @returns True when the reading is younger than {@link FRESH_MS}.
 */
export function isFresh(latest: Pick<Measurement, "ts_unix_ms"> | null | undefined, nowMs: number): boolean {
  return !!latest && nowMs - latest.ts_unix_ms < FRESH_MS;
}

/**
 * Build the context-aware health status of the home screen.
 *
 * @param input - The most important open alert, the latest reading and the current time.
 * @returns Tone, title and context line in plain Vietnamese.
 * @remarks
 * An open alert always wins (critical in red, others in amber). Without an
 * alert, the heart rate is read against the current activity so a high value
 * while walking still reads as normal, as the wireframe shows.
 */
export function healthStatus(input: {
  openAlert: Alert | null;
  latest: Measurement | null;
  nowMs: number;
}): HealthStatus {
  const { openAlert, latest, nowMs } = input;
  const fresh = isFresh(latest, nowMs);
  if (openAlert) {
    const nearAlert = !!latest && Math.abs(latest.ts_unix_ms - openAlert.raised_at_unix_ms) < FRESH_MS;
    const activity = latest && (nearAlert || fresh) ? latest.activity : null;
    const value = alertValueText(openAlert);
    const time = formatTime(openAlert.raised_at_unix_ms);
    return {
      tone: openAlert.severity === "critical" ? "alert" : "warn",
      title: alertTitle(openAlert, activity),
      context: `${value ? vi.status.alertContext(value, time) : vi.status.alertContextNoValue(time)} · ${vi.status.doctorWatching}`,
    };
  }
  if (!latest) {
    return { tone: "muted", title: vi.status.noData, context: vi.status.noDataContext };
  }
  if (!fresh) {
    return { tone: "muted", title: vi.status.stale, context: vi.status.staleContext(formatTime(latest.ts_unix_ms)) };
  }
  if (latest.hr_bpm === null) {
    return { tone: "ok", title: vi.status.normal, context: vi.status.noHr };
  }
  const context =
    latest.activity === "unknown"
      ? vi.status.hrNormal(latest.hr_bpm)
      : vi.status.hrFitsActivity(latest.hr_bpm, vi.activityPhrase[latest.activity]);
  return { tone: "ok", title: vi.status.normal, context };
}
