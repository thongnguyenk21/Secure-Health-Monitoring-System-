import type { Alert, Measurement, TokenPair } from "../../src/api/types";

/** Build an unsigned JWT with the given claims (the app never checks signatures). */
export function fakeJwt(claims: Record<string, unknown>): string {
  const enc = (o: unknown) =>
    btoa(String.fromCharCode(...new TextEncoder().encode(JSON.stringify(o))))
      .replace(/=+$/, "")
      .replace(/\+/g, "-")
      .replace(/\//g, "_");
  return `${enc({ alg: "EdDSA", typ: "JWT" })}.${enc(claims)}.c2ln`;
}

/** A token pair whose access token expires at `expMs`. */
export function fakePair(expMs: number, role = "patient", sub = "u-1", refresh = "r-1"): TokenPair {
  return {
    access_token: fakeJwt({ sub, role, exp: Math.floor(expMs / 1000) }),
    access_expires_at_unix_ms: expMs,
    refresh_token: refresh,
    refresh_expires_at_unix_ms: expMs + 30 * 86_400_000,
  };
}

export const DEVICE_ID = "9be2d431c9146b0535ae5db75c185ebf";

/** A raw reading. */
export function reading(over: Partial<Measurement> = {}): Measurement {
  return {
    device_id: DEVICE_ID,
    ts_unix_ms: Date.now(),
    hr_bpm: 92,
    spo2_pct: 97,
    activity: "walk",
    signal_quality: 80,
    ...over,
  };
}

/** An alert. */
export function alert(over: Partial<Alert> = {}): Alert {
  return {
    id: "a-1",
    device_id: DEVICE_ID,
    alert_type: "hr_high",
    severity: "warning",
    status: "open",
    raised_at_unix_ms: Date.now(),
    value: 154,
    acknowledged_at_unix_ms: null,
    acknowledged_by: null,
    ...over,
  };
}

/** A JSON response for a mocked fetch. */
export function json(body: unknown, status = 200): Response {
  return new Response(status === 204 ? null : JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}
