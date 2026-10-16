import { alertContextLine, alertStatusText, alertTitle, topOpenAlert } from "../src/features/alerts/alertText";
import { FRESH_MS, healthStatus, isFresh, toneStyle } from "../src/features/home/healthStatus";
import { colors } from "../src/theme/colors";
import { alert, reading } from "./helpers/fixtures";

const NOW = 1_800_000_000_000;

describe("healthStatus", () => {
  it("reads a walking heart rate as normal, in green", () => {
    const s = healthStatus({ openAlert: null, latest: reading({ ts_unix_ms: NOW - 1_000, hr_bpm: 92, activity: "walk" }), nowMs: NOW });
    expect(s).toEqual({ tone: "ok", title: "Sức khoẻ: bình thường", context: "Nhịp tim 92 hợp với lúc đi bộ" });
    expect(toneStyle(s.tone).color).toBe(colors.ok);
  });

  it("uses the activity in the context line", () => {
    const rest = healthStatus({ openAlert: null, latest: reading({ ts_unix_ms: NOW, hr_bpm: 64, activity: "rest" }), nowMs: NOW });
    expect(rest.context).toBe("Nhịp tim 64 hợp với lúc nghỉ");
    const run = healthStatus({ openAlert: null, latest: reading({ ts_unix_ms: NOW, hr_bpm: 141, activity: "run" }), nowMs: NOW });
    expect(run.context).toBe("Nhịp tim 141 hợp với lúc chạy");
    const unknown = healthStatus({ openAlert: null, latest: reading({ ts_unix_ms: NOW, hr_bpm: 70, activity: "unknown" }), nowMs: NOW });
    expect(unknown.context).toBe("Nhịp tim 70, trong mức bình thường");
  });

  it("shows a warning alert in amber with its value", () => {
    const open = alert({ alert_type: "hr_high", severity: "warning", value: 154, raised_at_unix_ms: NOW - 60_000 });
    const s = healthStatus({ openAlert: open, latest: reading({ ts_unix_ms: NOW, activity: "walk" }), nowMs: NOW });
    expect(s.tone).toBe("warn");
    expect(toneStyle(s.tone).color).toBe(colors.warn);
    expect(s.title).toBe("Nhịp tim nhanh");
    expect(s.context).toMatch(/^154 nhịp\/phút lúc \d\d:\d\d · bác sĩ đang theo dõi$/);
  });

  it("names a high heart rate at rest and shows critical alerts in red", () => {
    const open = alert({ alert_type: "hr_high", severity: "critical", raised_at_unix_ms: NOW });
    const s = healthStatus({ openAlert: open, latest: reading({ ts_unix_ms: NOW, activity: "rest" }), nowMs: NOW });
    expect(s.tone).toBe("alert");
    expect(toneStyle(s.tone).color).toBe(colors.alert);
    expect(s.title).toBe("Nhịp nhanh khi nghỉ");
  });

  it("describes low oxygen in plain words", () => {
    const open = alert({ alert_type: "spo2_low", value: 91, raised_at_unix_ms: NOW });
    const s = healthStatus({ openAlert: open, latest: null, nowMs: NOW });
    expect(s.title).toBe("Oxy máu giảm");
    expect(s.context).toMatch(/^còn 91 % lúc/);
  });

  it("says when readings are missing or old", () => {
    expect(healthStatus({ openAlert: null, latest: null, nowMs: NOW }).title).toBe("Chưa có số đo");
    const stale = healthStatus({ openAlert: null, latest: reading({ ts_unix_ms: NOW - FRESH_MS }), nowMs: NOW });
    expect(stale.tone).toBe("muted");
    expect(stale.title).toBe("Chưa nhận được số đo mới");
  });

  it("treats a reading as fresh for under two minutes", () => {
    expect(isFresh(reading({ ts_unix_ms: NOW - FRESH_MS + 1 }), NOW)).toBe(true);
    expect(isFresh(reading({ ts_unix_ms: NOW - FRESH_MS }), NOW)).toBe(false);
    expect(isFresh(null, NOW)).toBe(false);
  });
});

describe("alert text", () => {
  it("builds the emergency context line", () => {
    const a = alert({ alert_type: "hr_high", value: 154, raised_at_unix_ms: NOW });
    expect(alertContextLine(a, "walk")).toMatch(/^Lúc \d\d:\d\d · 154 nhịp\/phút khi đang đi bộ$/);
    expect(alertContextLine({ ...a, alert_type: "device_offline", value: null }, null)).toMatch(/^Lúc \d\d:\d\d$/);
  });

  it("tells who acknowledged an alert", () => {
    expect(alertStatusText(alert({ status: "open" }), "me")).toBe("Đang chờ xử lý");
    expect(alertStatusText(alert({ status: "acknowledged", acknowledged_by: "me" }), "me")).toBe("Bạn đã báo ổn");
    expect(alertStatusText(alert({ status: "acknowledged", acknowledged_by: "doc" }), "me")).toBe("Bác sĩ đã xem");
    expect(alertStatusText(alert({ status: "resolved", acknowledged_by: null }), "me")).toBe("Đã qua");
  });

  it("titles every alert type without technical words", () => {
    const types = ["hr_high", "hr_low", "spo2_low", "anomaly", "device_offline", "battery_low"] as const;
    for (const t of types) {
      expect(alertTitle({ alert_type: t })).not.toMatch(/SNN|model|Pi\b|firmware|buffer|gateway/i);
    }
  });

  it("picks the most severe open alert, newest first", () => {
    const list = [
      alert({ id: "old-warn", severity: "warning", raised_at_unix_ms: 1 }),
      alert({ id: "crit", severity: "critical", raised_at_unix_ms: 2 }),
      alert({ id: "new-warn", severity: "warning", raised_at_unix_ms: 3 }),
      alert({ id: "closed", severity: "critical", status: "resolved", raised_at_unix_ms: 4 }),
    ];
    expect(topOpenAlert(list)?.id).toBe("crit");
    expect(topOpenAlert([])).toBeNull();
  });
});
