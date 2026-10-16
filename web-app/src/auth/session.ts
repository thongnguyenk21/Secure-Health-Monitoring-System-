import { postPublic, SessionExpiredError } from "../api/http";
import type { TokenPair } from "../api/types";
import { decodeAccessToken, needsRefresh, refreshDelayMs, type AccessClaims } from "./token";

/** Tokens and decoded claims of the logged-in user. */
export interface Session {
  /** Token pair from the last login or refresh. */
  pair: TokenPair;
  /** Claims decoded from the access token. */
  claims: AccessClaims;
}

const STORAGE_KEY = "iomt.session";
// setTimeout overflows above 2^31 - 1 ms (about 24.8 days).
const MAX_TIMER_MS = 2_147_483_647;

let current: Session | null = null;
let refreshTimer: ReturnType<typeof setTimeout> | null = null;
let inflight: Promise<Session> | null = null;
const listeners = new Set<() => void>();

function emit(): void {
  for (const listener of listeners) listener();
}

function persist(session: Session | null): void {
  try {
    if (session) sessionStorage.setItem(STORAGE_KEY, JSON.stringify(session.pair));
    else sessionStorage.removeItem(STORAGE_KEY);
  } catch {
    // Storage can be disabled; the in-memory copy still works for this tab.
  }
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

function apply(pair: TokenPair | null): Session | null {
  current = pair ? { pair, claims: decodeAccessToken(pair.access_token) } : null;
  persist(current);
  schedule(current);
  emit();
  return current;
}

/**
 * Store a token pair from login or refresh as the current session.
 *
 * @param pair - Token pair from the backend.
 * @returns The new session.
 */
export function setSession(pair: TokenPair): Session {
  return apply(pair) as Session;
}

/** Forget the session in memory and in sessionStorage. */
export function clearSession(): void {
  inflight = null;
  apply(null);
}

/**
 * Read the current session.
 *
 * @returns The session, or null when logged out.
 */
export function getSession(): Session | null {
  return current;
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
 * @throws {@link SessionExpiredError} When there is no session or the backend refuses.
 * @remarks
 * Concurrent callers share one request: reusing an old refresh token revokes
 * the whole session on the backend, so two parallel refreshes must never run.
 */
export function refreshSession(): Promise<Session> {
  if (inflight) return inflight;
  const session = current;
  if (!session) return Promise.reject(new SessionExpiredError("no session"));
  inflight = postPublic<TokenPair>("/auth/refresh", { refresh_token: session.pair.refresh_token })
    .then((pair) => setSession(pair))
    .catch(() => {
      clearSession();
      throw new SessionExpiredError("refresh refused");
    })
    .finally(() => {
      inflight = null;
    });
  return inflight;
}

/**
 * Get an access token that is valid for at least the refresh lead time.
 *
 * @returns The access token.
 * @throws {@link SessionExpiredError} When logged out or the refresh fails.
 */
export async function freshAccessToken(): Promise<string> {
  const session = current;
  if (!session) throw new SessionExpiredError("no session");
  if (needsRefresh(expiresAt(session), Date.now())) {
    return (await refreshSession()).pair.access_token;
  }
  return session.pair.access_token;
}

/** Restore the session saved in sessionStorage, dropping it when unreadable or expired. */
export function restoreSession(): void {
  let pair: TokenPair | null = null;
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    pair = raw ? (JSON.parse(raw) as TokenPair) : null;
  } catch {
    pair = null;
  }
  if (!pair || pair.refresh_expires_at_unix_ms <= Date.now()) {
    clearSession();
    return;
  }
  try {
    setSession(pair);
  } catch {
    clearSession();
  }
}
