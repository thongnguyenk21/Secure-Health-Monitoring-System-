import type { PatientSummary, TokenPair } from "../api/types";

function b64url(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export function makeJwt(payload: Record<string, unknown>): string {
  return `${b64url(JSON.stringify({ alg: "EdDSA", kid: "k1" }))}.${b64url(JSON.stringify(payload))}.c2ln`;
}

export function makePair(role: "patient" | "clinician" | "admin", expiresInMs = 15 * 60_000): TokenPair {
  const exp = Date.now() + expiresInMs;
  return {
    access_token: makeJwt({ sub: "11111111-1111-4111-8111-111111111111", role, exp: Math.floor(exp / 1000) }),
    access_expires_at_unix_ms: exp,
    refresh_token: "refresh-1",
    refresh_expires_at_unix_ms: Date.now() + 30 * 24 * 3600_000,
  };
}

let seq = 0;
export function makePatient(overrides: Partial<PatientSummary>): PatientSummary {
  seq += 1;
  return {
    user_id: `00000000-0000-4000-8000-${String(seq).padStart(12, "0")}`,
    subject_code: `BN-${String(seq).padStart(3, "0")}`,
    display_name: null,
    device_id: "a1f3".padEnd(32, "0"),
    status: "normal",
    open_alert_count: 0,
    top_alert: null,
    latest: null,
    hrv_rmssd_ms: null,
    last_seen_unix_ms: Date.now(),
    ...overrides,
  };
}
