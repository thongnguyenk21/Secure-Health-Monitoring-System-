import type { Role } from "../api/types";

/** How long before the access token expires the app refreshes it, in ms. */
export const REFRESH_LEAD_MS = 60_000;

/** Claims of the access token that the web app reads. */
export interface AccessClaims {
  /** User id. */
  sub: string;
  /** Role of the user. */
  role: Role;
  /** Expiry, UTC epoch seconds. */
  exp: number;
}

/** Raised when a token is not a well-formed JWT with the expected claims. */
export class TokenFormatError extends Error {
  /** @param message - What is wrong with the token. */
  constructor(message: string) {
    super(message);
    this.name = "TokenFormatError";
  }
}

const ROLES: readonly Role[] = ["patient", "clinician", "admin"];

/**
 * Decode one base64url segment into a UTF-8 string.
 *
 * @param segment - Base64url text without padding.
 * @returns The decoded string.
 */
export function base64UrlDecode(segment: string): string {
  const base64 = segment.replace(/-/g, "+").replace(/_/g, "/");
  const padded = base64 + "=".repeat((4 - (base64.length % 4)) % 4);
  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

/**
 * Read the claims of a JWT access token without verifying its signature.
 *
 * @param token - Compact JWT (`header.payload.signature`).
 * @returns The `sub`, `role` and `exp` claims.
 * @throws {@link TokenFormatError} When the token or a claim is malformed.
 * @remarks
 * The signature is checked by the backend on every request; the client only
 * needs the claims to route the user and schedule the refresh.
 */
export function decodeAccessToken(token: string): AccessClaims {
  const parts = token.split(".");
  if (parts.length !== 3 || !parts[1]) {
    throw new TokenFormatError("token is not a compact JWT");
  }
  let payload: unknown;
  try {
    payload = JSON.parse(base64UrlDecode(parts[1]));
  } catch {
    throw new TokenFormatError("token payload is not base64url JSON");
  }
  if (typeof payload !== "object" || payload === null) {
    throw new TokenFormatError("token payload is not an object");
  }
  const { sub, role, exp } = payload as Record<string, unknown>;
  if (typeof sub !== "string" || typeof exp !== "number") {
    throw new TokenFormatError("token lacks sub or exp");
  }
  if (typeof role !== "string" || !ROLES.includes(role as Role)) {
    throw new TokenFormatError("token role is unknown");
  }
  return { sub, role: role as Role, exp };
}

/**
 * Compute how long to wait before refreshing the access token.
 *
 * @param expiresAtMs - Access token expiry, UTC epoch ms.
 * @param nowMs - Current time, UTC epoch ms.
 * @param leadMs - Margin before expiry; defaults to {@link REFRESH_LEAD_MS}.
 * @returns Delay in ms, never negative (0 means refresh now).
 */
export function refreshDelayMs(expiresAtMs: number, nowMs: number, leadMs: number = REFRESH_LEAD_MS): number {
  return Math.max(0, expiresAtMs - leadMs - nowMs);
}

/**
 * Tell whether the access token should be refreshed before it is used.
 *
 * @param expiresAtMs - Access token expiry, UTC epoch ms.
 * @param nowMs - Current time, UTC epoch ms.
 * @returns True when the token expires within {@link REFRESH_LEAD_MS}.
 */
export function needsRefresh(expiresAtMs: number, nowMs: number): boolean {
  return refreshDelayMs(expiresAtMs, nowMs) === 0;
}
