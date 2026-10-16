import { describe, expect, it } from "vitest";
import type { Alert } from "../api/types";
import { makePatient } from "../test/fixtures";
import { activityLabel, deviceStatusView, patientStatusColor, patientStatusView } from "./status";

const NOW = 1_700_000_000_000;

function alert(type: Alert["alert_type"], severity: Alert["severity"]): Alert {
  return {
    id: "a1",
    device_id: "a".repeat(32),
    alert_type: type,
    severity,
    status: "open",
    raised_at_unix_ms: NOW - 60_000,
    value: 152,
  };
}

describe("patientStatusView", () => {
  it("maps each status to its wireframe colour", () => {
    expect(patientStatusColor("critical")).toBe("var(--alert)");
    expect(patientStatusColor("warning")).toBe("var(--warn)");
    expect(patientStatusColor("offline")).toBe("var(--ext)");
    expect(patientStatusColor("normal")).toBe("var(--ok)");
  });

  it("labels critical and warning patients by their top alert type", () => {
    expect(patientStatusView(makePatient({ status: "critical", top_alert: alert("hr_high", "critical") }), NOW)).toEqual({
      label: "Nhịp tim cao",
      color: "var(--alert)",
    });
    expect(patientStatusView(makePatient({ status: "warning", top_alert: alert("spo2_low", "warning") }), NOW).label).toBe(
      "SpO₂ giảm",
    );
  });

  it("labels offline patients with the minutes since the last upload", () => {
    const view = patientStatusView(makePatient({ status: "offline", last_seen_unix_ms: NOW - 12 * 60_000 }), NOW);
    expect(view.label).toBe("Mất kết nối 12 phút");
  });

  it("labels normal patients with their activity", () => {
    const latest = {
      device_id: "a".repeat(32),
      ts_unix_ms: NOW,
      hr_bpm: 96,
      spo2_pct: 98,
      activity: "walk" as const,
      signal_quality: 90,
    };
    expect(patientStatusView(makePatient({ status: "normal", latest }), NOW).label).toBe("Bình thường · đi bộ");
    expect(activityLabel("rest")).toBe("Nghỉ");
    expect(activityLabel("run")).toBe("Chạy");
    expect(activityLabel(undefined)).toBe("Không rõ");
  });
});

describe("deviceStatusView", () => {
  const base = { device_id: "a".repeat(32), kind: "wearable" as const };
  it("distinguishes online, offline, pending and revoked", () => {
    expect(deviceStatusView({ ...base, status: "active", last_seen_unix_ms: NOW - 60_000 }, NOW).label).toBe("trực tuyến");
    expect(deviceStatusView({ ...base, status: "active", last_seen_unix_ms: NOW - 6 * 60_000 }, NOW).label).toBe(
      "mất kết nối",
    );
    expect(deviceStatusView({ ...base, status: "pending" }, NOW).label).toBe("chờ kích hoạt");
    expect(deviceStatusView({ ...base, status: "revoked" }, NOW).label).toBe("đã thu hồi");
  });
});

describe("formatWeekday", () => {
  it("shortens Vietnamese weekdays to T2…T7 and CN", async () => {
    const { formatWeekday } = await import("./format");
    // 2026-10-14 is a Wednesday (T4); 2026-10-18 is a Sunday.
    expect(formatWeekday(new Date(2026, 9, 14, 12).getTime())).toBe("T4");
    expect(formatWeekday(new Date(2026, 9, 18, 12).getTime())).toBe("CN");
  });
});
