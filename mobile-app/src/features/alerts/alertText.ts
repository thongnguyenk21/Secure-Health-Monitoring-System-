import type { Activity, Alert, Severity } from "../../api/types";
import { vi } from "../../i18n/vi";
import { colors } from "../../theme/colors";
import { formatTime } from "../../utils/format";

/**
 * Plain-words title of an alert for the patient.
 *
 * @param alert - The alert.
 * @param activity - Activity around the alert time, when known.
 * @returns For example "Nhịp tim nhanh" or "Nhịp nhanh khi nghỉ".
 */
export function alertTitle(alert: Pick<Alert, "alert_type">, activity?: Activity | null): string {
  if (alert.alert_type === "hr_high" && activity === "rest") return vi.alertTitle.hr_high_rest;
  return vi.alertTitle[alert.alert_type];
}

/**
 * The measured value of an alert in everyday words.
 *
 * @param alert - The alert.
 * @returns For example "154 nhịp/phút", or null when the alert carries no value.
 */
export function alertValueText(alert: Pick<Alert, "alert_type" | "value">): string | null {
  if (alert.value === null || alert.value === undefined) return null;
  const v = Math.round(alert.value);
  switch (alert.alert_type) {
    case "hr_high":
    case "hr_low":
    case "anomaly":
      return vi.alertValue.hr(v);
    case "spo2_low":
      return vi.alertValue.spo2(v);
    default:
      return null;
  }
}

/**
 * Context line under the title of the emergency screen.
 *
 * @param alert - The alert.
 * @param activity - Activity around the alert time, when known.
 * @returns For example "Lúc 14:32 · 154 nhịp/phút khi đang đi bộ".
 */
export function alertContextLine(alert: Pick<Alert, "alert_type" | "value" | "raised_at_unix_ms">, activity?: Activity | null): string {
  const parts = [vi.emergency.at(formatTime(alert.raised_at_unix_ms))];
  const value = alertValueText(alert);
  const during = activity && activity !== "unknown" ? vi.emergency.duringActivity(vi.activityPhrase[activity]) : null;
  if (value && during) parts.push(`${value} ${during}`);
  else if (value) parts.push(value);
  else if (during) parts.push(during);
  return parts.join(" · ");
}

/**
 * Status line of an alert for the timeline and the alert list.
 *
 * @param alert - The alert.
 * @param myUserId - Id of the signed-in patient, to tell who acknowledged it.
 * @returns For example "Bác sĩ đã xem" or "Bạn đã báo ổn".
 */
export function alertStatusText(
  alert: Pick<Alert, "status" | "acknowledged_by">,
  myUserId: string | null,
): string {
  if (alert.status === "open") return vi.alertStatus.open;
  if (alert.status === "resolved" && !alert.acknowledged_by) return vi.alertStatus.resolved;
  if (alert.acknowledged_by && alert.acknowledged_by === myUserId) return vi.alertStatus.acknowledgedByMe;
  return vi.alertStatus.acknowledgedByDoctor;
}

/**
 * Colour of a severity: red for critical, amber for warning, grey for info.
 *
 * @param severity - Alert severity.
 * @returns A colour value from the theme.
 */
export function severityColor(severity: Severity): string {
  if (severity === "critical") return colors.alert;
  if (severity === "warning") return colors.warn;
  return colors.muted;
}

const SEVERITY_RANK: Record<Severity, number> = { critical: 2, warning: 1, info: 0 };

/**
 * The open alert to show on the home screen: most severe, then newest.
 *
 * @param alerts - Alerts of the patient.
 * @returns The alert, or null when none is open.
 */
export function topOpenAlert(alerts: readonly Alert[]): Alert | null {
  let best: Alert | null = null;
  for (const a of alerts) {
    if (a.status !== "open") continue;
    if (
      !best ||
      SEVERITY_RANK[a.severity] > SEVERITY_RANK[best.severity] ||
      (SEVERITY_RANK[a.severity] === SEVERITY_RANK[best.severity] && a.raised_at_unix_ms > best.raised_at_unix_ms)
    ) {
      best = a;
    }
  }
  return best;
}

/**
 * Order alerts for the alert list: open ones first, then newest first.
 *
 * @param alerts - Alerts in any order.
 * @returns A new sorted array.
 */
export function sortAlertsOpenFirst(alerts: readonly Alert[]): Alert[] {
  return [...alerts].sort((a, b) => {
    const openA = a.status === "open" ? 1 : 0;
    const openB = b.status === "open" ? 1 : 0;
    if (openA !== openB) return openB - openA;
    return b.raised_at_unix_ms - a.raised_at_unix_ms;
  });
}
