import type {
  Activity,
  AlertStatus,
  AlertType,
  ComponentStatus,
  Device,
  PatientStatus,
  PatientSummary,
  Severity,
} from "../api/types";
import { vi } from "../i18n/vi";

/** CSS colour tokens used for status dots. */
export type StatusColor = "var(--alert)" | "var(--warn)" | "var(--ext)" | "var(--ok)" | "var(--line)";

/** A device without an upload for longer than this is offline, in ms. */
export const OFFLINE_AFTER_MS = 5 * 60_000;

const PATIENT_COLOR: Record<PatientStatus, StatusColor> = {
  critical: "var(--alert)",
  warning: "var(--warn)",
  offline: "var(--ext)",
  normal: "var(--ok)",
  no_device: "var(--line)",
};

/**
 * Colour of the status dot of a patient.
 *
 * @param status - Patient priority level.
 * @returns The CSS colour.
 */
export function patientStatusColor(status: PatientStatus): StatusColor {
  return PATIENT_COLOR[status];
}

/**
 * Vietnamese label of an activity class.
 *
 * @param activity - Activity from a measurement, or nothing.
 * @returns The label.
 */
export function activityLabel(activity: Activity | null | undefined): string {
  return vi.activity[activity ?? "unknown"];
}

/**
 * Vietnamese label of an alert type.
 *
 * @param type - Alert type.
 * @returns The label.
 */
export function alertTypeLabel(type: AlertType): string {
  return vi.alertType[type];
}

/**
 * Vietnamese label of an alert severity.
 *
 * @param severity - Alert severity.
 * @returns The label.
 */
export function severityLabel(severity: Severity): string {
  return vi.severity[severity];
}

/**
 * Vietnamese label of an alert status.
 *
 * @param status - Alert status.
 * @returns The label.
 */
export function alertStatusLabel(status: AlertStatus): string {
  return vi.alertStatus[status];
}

/**
 * Colour of an alert severity.
 *
 * @param severity - Alert severity.
 * @returns The CSS colour.
 */
export function severityColor(severity: Severity): StatusColor {
  if (severity === "critical") return "var(--alert)";
  if (severity === "warning") return "var(--warn)";
  return "var(--ext)";
}

/** Label and colour of a patient row. */
export interface PatientStatusView {
  /** Vietnamese status text. */
  label: string;
  /** Status dot colour. */
  color: StatusColor;
}

/**
 * Describe the status of a patient for the patient list.
 *
 * @param patient - Patient summary.
 * @param nowMs - Current time, UTC epoch ms.
 * @returns The label and dot colour.
 */
export function patientStatusView(patient: PatientSummary, nowMs: number): PatientStatusView {
  const color = patientStatusColor(patient.status);
  switch (patient.status) {
    case "critical":
    case "warning":
      return { color, label: patient.top_alert ? alertTypeLabel(patient.top_alert.alert_type) : vi.status.hasAlert };
    case "offline": {
      if (patient.last_seen_unix_ms === null || patient.last_seen_unix_ms === undefined) {
        return { color, label: vi.status.offline };
      }
      const minutes = Math.max(1, Math.floor((nowMs - patient.last_seen_unix_ms) / 60_000));
      return { color, label: vi.status.offlineMinutes(minutes) };
    }
    case "normal":
      return { color, label: vi.status.normal(activityLabel(patient.latest?.activity).toLowerCase()) };
    case "no_device":
      return { color, label: vi.status.noDevice };
  }
}

/**
 * Describe the connection state of a device for the admin tables.
 *
 * @param device - Device.
 * @param nowMs - Current time, UTC epoch ms.
 * @returns The label and dot colour.
 */
export function deviceStatusView(device: Device, nowMs: number): PatientStatusView {
  if (device.status === "revoked") return { label: vi.devices.revoked, color: "var(--line)" };
  if (device.status === "pending") return { label: vi.devices.pending, color: "var(--warn)" };
  const seen = device.last_seen_unix_ms;
  if (seen !== null && seen !== undefined && nowMs - seen < OFFLINE_AFTER_MS) {
    return { label: vi.devices.online, color: "var(--ok)" };
  }
  return { label: vi.devices.offline, color: "var(--ext)" };
}

/**
 * Colour of a system component state.
 *
 * @param state - Component state.
 * @returns The CSS colour.
 */
export function componentColor(state: ComponentStatus["state"]): StatusColor {
  if (state === "ok") return "var(--ok)";
  if (state === "degraded") return "var(--warn)";
  if (state === "down") return "var(--alert)";
  return "var(--ext)";
}
