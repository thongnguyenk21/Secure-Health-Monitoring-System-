import { ApiError, postPublic, SessionExpiredError } from "../../api/http";
import type { TokenPair } from "../../api/types";
import { decodeAccessToken, needsRefresh, refreshDelayMs, type AccessClaims } from "./token";
import { createPlatformTokenStore, type TokenStore } from "./tokenStore";

/** Tokens and decoded claims of the logged-in user. */
export interface Session {
  /** Token pair from the last login or refresh. */
  pair: TokenPair;
  /** Claims decoded from the access token. */
  claims: AccessClaims;
}

/** Why the last session ended, shown once on the login screen. */
export type EndReason = "logout" | "expired" | null;

/** Snapshot of the session state for React (`useSyncExternalStore`). */
export interface SessionState {
  /** True until the stored session has been read at start-up. */
  restoring: boolean;
  /** Current session, or null when logged out. */
  session: Session | null;
  /** Why the previous session ended. */
  endReason: EndReason;
}

// setTimeout overflows above 2^31 - 1 ms (about 24.8 days).
const MAX_TIMER_MS = 2_147_483_647;

let store: TokenStore = createPlatformTokenStore();
let state: SessionState = { restoring: true, session: null, endReason: null };
let refreshTimer: ReturnType<typeof setTimeout> | null = null;
let inflight: Promise<Session> | null = null;
const listeners = new Set<() => void>();

function emit(next: SessionState): void {
  state = next;
  for (const listener of listeners) listener();
}

function expiresAt(session: Session): number {
  return session.pair.access_expires_at_unix_ms || session.claims.exp * 1000;
}

function schedule(session: Session | null): void {
  if (refreshTimer) clearTimeout(refreshTimer);
  refreshTimer = null;
  if (!session) return;
  const delay = Math.min(refreshDelayMs(expiresAt(session), Date.now()), MAX_TIMER_MS);
  refreshTimer = setTimeout(() => {
    refreshSession().catch(() => undefined);
  }, delay);
}

function apply(pair: TokenPair): Session {
  const session = { pair, claims: decodeAccessToken(pair.access_token) };
  store.save(JSON.stringify(pair)).catch(() => undefined);
  schedule(session);
  emit({ restoring: false, session, endReason: null });
  return session;
}

/**
 * Replace the token store (tests only).
 *
 * @param next - Store to use from now on.
 */
export function setTokenStoreForTesting(next: TokenStore): void {
  store = next;
}

/**
 * Store a token pair from login or refresh as the current session.
 *
 * @param pair - Token pair from the backend.
 * @returns The new session.
 * @throws {@link TokenFormatError} When the access token is malformed.
 */
export function setSession(pair: TokenPair): Session {
  return apply(pair);
}

/**
 * Forget the session in memory and in the token store.
 *
 * @param reason - Why the session ended; shown once on the login screen.
 */
export function clearSession(reason: EndReason = "logout"): void {
  inflight = null;
  schedule(null);
  store.clear().catch(() => undefined);
  emit({ restoring: false, session: null, endReason: reason });
}

/**
 * Read the current session state.
 *
 * @returns The state snapshot; the same object until the next change.
 */
export function getSessionState(): SessionState {
  return state;
}

/**
 * Read the current session.
 *
 * @returns The session, or null when logged out.
 */
export function getSession(): Session | null {
  return state.session;
}

/**
 * Subscribe to session changes (for `useSyncExternalStore`).
 *
 * @param listener - Called after every change.
 * @returns A function that removes the listener.
 */
export function subscribeSession(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * Rotate the token pair with the refresh token.
 *
 * @returns The refreshed session.
 * @throws {@link SessionExpiredError} When there is no session or the backend refuses the token.
 * @throws {@link NetworkError} When the backend cannot be reached; the session is kept.
 * @remarks
 * Single-flight: concurrent callers share one request, because reusing an old
 * refresh token makes the backend revoke the whole session family.
 */
export function refreshSession(): Promise<Session> {
  if (inflight) return inflight;
  const session = state.session;
  if (!session) return Promise.reject(new SessionExpiredError("no session"));
  inflight = postPublic<TokenPair>("/auth/refresh", { refresh_token: session.pair.refresh_token })
    .then((pair) => setSession(pair))
    .catch((err: unknown) => {
      if (err instanceof ApiError && (err.status === 401 || err.status === 400)) {
        clearSession("expired");
        throw new SessionExpiredError("refresh refused");
      }
      throw err;
    })
    .finally(() => {
      inflight = null;
    });
  return inflight;
}

/**
 * Get an access token that stays valid for at least the refresh lead time.
 *
 * @returns The access token.
 * @throws {@link SessionExpiredError} When logged out or the refresh is refused.
 */
export async function freshAccessToken(): Promise<string> {
  const session = state.session;
  if (!session) throw new SessionExpiredError("no session");
  if (needsRefresh(expiresAt(session), Date.now())) {
    return (await refreshSession()).pair.access_token;
  }
  return session.pair.access_token;
}

function parsePair(raw: string | null): TokenPair | null {
  if (!raw) return null;
  try {
    const value: unknown = JSON.parse(raw);
    if (typeof value !== "object" || value === null) return null;
    const pair = value as TokenPair;
    return typeof pair.access_token === "string" && typeof pair.refresh_token === "string" ? pair : null;
  } catch {
    return null;
  }
}

/**
 * Restore the stored session at start-up, dropping it when unreadable or expired.
 *
 * @returns A promise that settles once `restoring` is false.
 */
export async function restoreSession(): Promise<void> {
  let pair: TokenPair | null = null;
  try {
    pair = parsePair(await store.load());
  } catch {
    pair = null;
  }
  if (!pair || pair.refresh_expires_at_unix_ms <= Date.now()) {
    clearSession(null);
    return;
  }
  try {
    setSession(pair);
  } catch {
    clearSession(null);
  }
}
